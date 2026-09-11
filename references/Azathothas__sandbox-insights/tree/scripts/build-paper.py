#!/usr/bin/env python3
"""Build the canonical paper Markdown into LaTeX and an arXiv-style PDF."""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

from pypdf import PdfReader
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_JUSTIFY, TA_LEFT
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (
    BaseDocTemplate,
    Frame,
    FrameBreak,
    HRFlowable,
    KeepTogether,
    ListFlowable,
    ListItem,
    LongTable,
    PageTemplate,
    Paragraph,
    Preformatted,
    Spacer,
    TableStyle,
)


@dataclass
class Block:
    kind: str
    value: object
    level: int = 0


def parse_markdown(text: str) -> list[Block]:
    lines = text.splitlines()
    blocks: list[Block] = []
    paragraph: list[str] = []
    i = 0

    def flush_paragraph() -> None:
        if paragraph:
            blocks.append(Block("paragraph", " ".join(x.strip() for x in paragraph)))
            paragraph.clear()

    while i < len(lines):
        line = lines[i]
        if line.startswith("```"):
            flush_paragraph()
            language = line[3:].strip()
            code: list[str] = []
            i += 1
            while i < len(lines) and not lines[i].startswith("```"):
                code.append(lines[i])
                i += 1
            blocks.append(Block("code", (language, "\n".join(code))))
        elif re.match(r"^#{2,4} ", line):
            flush_paragraph()
            marks, title = line.split(" ", 1)
            blocks.append(Block("heading", title.strip(), len(marks)))
        elif line.startswith("|") and line.rstrip().endswith("|"):
            flush_paragraph()
            table_lines: list[str] = []
            while i < len(lines) and lines[i].startswith("|") and lines[i].rstrip().endswith("|"):
                table_lines.append(lines[i])
                i += 1
            i -= 1
            rows: list[list[str]] = []
            for raw in table_lines:
                cells = [c.strip() for c in raw.strip().strip("|").split("|")]
                if all(re.fullmatch(r":?-{3,}:?", c) for c in cells):
                    continue
                rows.append(cells)
            blocks.append(Block("table", rows))
        elif re.match(r"^- ", line):
            flush_paragraph()
            items: list[str] = []
            while i < len(lines) and re.match(r"^- ", lines[i]):
                items.append(lines[i][2:].strip())
                i += 1
            i -= 1
            blocks.append(Block("list", ("bullet", items)))
        elif re.match(r"^\d+\. ", line):
            flush_paragraph()
            items = []
            while i < len(lines) and re.match(r"^\d+\. ", lines[i]):
                items.append(re.sub(r"^\d+\. ", "", lines[i]).strip())
                i += 1
            i -= 1
            blocks.append(Block("list", ("number", items)))
        elif not line.strip():
            flush_paragraph()
        else:
            paragraph.append(line)
        i += 1
    flush_paragraph()
    return blocks


def split_abstract(blocks: list[Block]) -> tuple[list[Block], list[Block]]:
    abstract: list[Block] = []
    body: list[Block] = []
    in_abstract = False
    seen = False
    for block in blocks:
        if block.kind == "heading" and block.level == 2 and block.value == "Abstract":
            in_abstract = True
            seen = True
            continue
        if in_abstract and block.kind == "heading" and block.level == 2:
            in_abstract = False
        if in_abstract:
            abstract.append(block)
        else:
            body.append(block)
    if not seen or not abstract:
        raise ValueError("paper must contain a non-empty ## Abstract section")
    return abstract, body


def register_fonts() -> tuple[str, str, str, str]:
    candidates = [
        (
            Path("C:/Windows/Fonts/times.ttf"),
            Path("C:/Windows/Fonts/timesbd.ttf"),
            Path("C:/Windows/Fonts/timesi.ttf"),
            Path("C:/Windows/Fonts/consola.ttf"),
        ),
        (
            Path("/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"),
            Path("/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"),
            Path("/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf"),
            Path("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"),
        ),
    ]
    for regular, bold, italic, mono in candidates:
        if all(p.exists() for p in (regular, bold, italic, mono)):
            pdfmetrics.registerFont(TTFont("PaperSerif", str(regular)))
            pdfmetrics.registerFont(TTFont("PaperSerif-Bold", str(bold)))
            pdfmetrics.registerFont(TTFont("PaperSerif-Italic", str(italic)))
            pdfmetrics.registerFont(TTFont("PaperMono", str(mono)))
            pdfmetrics.registerFontFamily(
                "PaperSerif",
                normal="PaperSerif",
                bold="PaperSerif-Bold",
                italic="PaperSerif-Italic",
                boldItalic="PaperSerif-Bold",
            )
            return "PaperSerif", "PaperSerif-Bold", "PaperSerif-Italic", "PaperMono"
    return "Times-Roman", "Times-Bold", "Times-Italic", "Courier"


def inline_pdf(text: str, mono_font: str) -> str:
    escaped = html.escape(text, quote=False)
    escaped = re.sub(
        r"`([^`]+)`",
        lambda m: f'<font name="{mono_font}" size="7.1">{m.group(1)}</font>',
        escaped,
    )
    escaped = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", escaped)
    escaped = re.sub(r"(?<!\*)\*([^*]+)\*(?!\*)", r"<i>\1</i>", escaped)
    return escaped


def latex_escape_plain(text: str) -> str:
    replacements = {
        "\\": r"\textbackslash{}",
        "&": r"\&",
        "%": r"\%",
        "$": r"\$",
        "#": r"\#",
        "_": r"\_",
        "{": r"\{",
        "}": r"\}",
        "~": r"\textasciitilde{}",
        "^": r"\textasciicircum{}",
    }
    out = "".join(replacements.get(ch, ch) for ch in text)
    out = out.replace("→", r"$\rightarrow$")
    out = out.replace("×", r"$\times$")
    out = out.replace("≤", r"$\leq$")
    out = out.replace("≥", r"$\geq$")
    out = out.replace("—", "---").replace("–", "--")
    out = out.replace("“", "``").replace("”", "''").replace("’", "'")
    return out


def inline_latex(text: str) -> str:
    parts = re.split(r"(`[^`]*`|\*\*.*?\*\*|(?<!\*)\*[^*]+\*(?!\*))", text)
    rendered: list[str] = []
    for part in parts:
        if not part:
            continue
        if part.startswith("`") and part.endswith("`"):
            rendered.append(r"\texttt{" + latex_escape_plain(part[1:-1]) + "}")
        elif part.startswith("**") and part.endswith("**"):
            rendered.append(r"\textbf{" + latex_escape_plain(part[2:-2]) + "}")
        elif part.startswith("*") and part.endswith("*"):
            rendered.append(r"\emph{" + latex_escape_plain(part[1:-1]) + "}")
        else:
            rendered.append(latex_escape_plain(part))
    return "".join(rendered)


def write_latex(metadata: dict, abstract: list[Block], body: list[Block], output: Path) -> None:
    lines = [
        r"\documentclass[10pt,twocolumn]{article}",
        r"\usepackage[letterpaper,margin=0.7in,columnsep=0.22in]{geometry}",
        r"\usepackage[T1]{fontenc}",
        r"\usepackage[utf8]{inputenc}",
        r"\usepackage{microtype}",
        r"\usepackage{booktabs,tabularx,array}",
        r"\usepackage{enumitem}",
        r"\usepackage[hidelinks]{hyperref}",
        r"\usepackage{xurl}",
        r"\setlength{\parindent}{1em}",
        r"\setlength{\parskip}{0pt}",
        r"\setlist{nosep,leftmargin=*}",
        r"\title{" + inline_latex(metadata["title"]) + "}",
        r"\author{" + inline_latex(metadata["author"]) + "}",
        r"\date{" + inline_latex(metadata["date"]) + "}",
        r"\begin{document}",
        r"\maketitle",
        r"\begin{abstract}",
    ]
    for block in abstract:
        if block.kind == "paragraph":
            lines.append(inline_latex(str(block.value)) + "\n")
    lines.extend([r"\end{abstract}", r"\small"])

    for block in body:
        if block.kind == "heading":
            command = {2: "section", 3: "subsection", 4: "subsubsection"}.get(block.level, "paragraph")
            title = re.sub(r"^\d+(?:\.\d+)*\.\s*", "", str(block.value))
            lines.append(f"\\{command}{{{inline_latex(title)}}}")
        elif block.kind == "paragraph":
            lines.append(inline_latex(str(block.value)) + "\n")
        elif block.kind == "code":
            _, code = block.value
            lines.extend([r"\begin{verbatim}", str(code), r"\end{verbatim}"])
        elif block.kind == "list":
            list_type, items = block.value
            env = "enumerate" if list_type == "number" else "itemize"
            lines.append(f"\\begin{{{env}}}")
            lines.extend(r"\item " + inline_latex(item) for item in items)
            lines.append(f"\\end{{{env}}}")
        elif block.kind == "table":
            rows = block.value
            if not rows:
                continue
            ncols = max(len(row) for row in rows)
            spec = "".join([r">{\raggedright\arraybackslash}X" for _ in range(ncols)])
            lines.extend([
                r"\begin{table*}[t]",
                r"\centering\footnotesize",
                r"\begin{tabularx}{\textwidth}{" + spec + "}",
                r"\toprule",
            ])
            for row_index, row in enumerate(rows):
                cells = row + [""] * (ncols - len(row))
                if row_index == 0:
                    rendered = [r"\textbf{" + inline_latex(c) + "}" for c in cells]
                else:
                    rendered = [inline_latex(c) for c in cells]
                lines.append(" & ".join(rendered) + r" \\")
                if row_index == 0:
                    lines.append(r"\midrule")
            lines.extend([r"\bottomrule", r"\end{tabularx}", r"\end{table*}"])
    lines.extend([r"\end{document}", ""])
    output.write_text("\n".join(lines), encoding="utf-8")


class PaperDocTemplate(BaseDocTemplate):
    def __init__(self, filename: str, metadata: dict, **kwargs: object) -> None:
        super().__init__(filename, **kwargs)
        self.metadata_values = metadata

    def beforeDocument(self) -> None:
        self.canv.setTitle(self.metadata_values["title"])
        self.canv.setAuthor(self.metadata_values["author"])
        self.canv.setSubject(self.metadata_values["subject"])
        self.canv.setKeywords(", ".join(self.metadata_values["keywords"]))

    def afterPage(self) -> None:
        decorator = getattr(self, "page_decorator", None)
        if decorator is not None:
            decorator(self.canv, self)


def build_pdf(metadata: dict, abstract: list[Block], body: list[Block], output: Path) -> None:
    regular, bold, italic, mono = register_fonts()
    page_w, page_h = letter
    margin_x = 0.70 * inch
    margin_bottom = 0.55 * inch
    margin_top = 0.72 * inch
    gap = 0.22 * inch
    usable_w = page_w - 2 * margin_x
    col_w = (usable_w - gap) / 2

    styles = getSampleStyleSheet()
    body_style = ParagraphStyle(
        "PaperBody",
        parent=styles["BodyText"],
        fontName=regular,
        fontSize=9.15,
        leading=11.15,
        alignment=TA_JUSTIFY,
        firstLineIndent=9,
        spaceAfter=3.8,
        allowWidows=0,
        allowOrphans=0,
    )
    abstract_style = ParagraphStyle(
        "PaperAbstract",
        parent=body_style,
        fontSize=9.15,
        leading=11.2,
        leftIndent=0.30 * inch,
        rightIndent=0.30 * inch,
        firstLineIndent=0,
        spaceAfter=4,
    )
    h1 = ParagraphStyle(
        "PaperH1",
        fontName=bold,
        fontSize=10.8,
        leading=12.2,
        spaceBefore=7,
        spaceAfter=4,
        keepWithNext=True,
        textColor=colors.HexColor("#111827"),
    )
    h2 = ParagraphStyle(
        "PaperH2",
        fontName=bold,
        fontSize=9.2,
        leading=10.5,
        spaceBefore=5,
        spaceAfter=2.5,
        keepWithNext=True,
        textColor=colors.HexColor("#243B53"),
    )
    h3 = ParagraphStyle(
        "PaperH3",
        fontName=italic,
        fontSize=8.6,
        leading=10,
        spaceBefore=4,
        spaceAfter=2,
        keepWithNext=True,
    )
    title_style = ParagraphStyle(
        "PaperTitle",
        fontName=bold,
        fontSize=18.5,
        leading=21.5,
        alignment=TA_CENTER,
        spaceAfter=8,
        textColor=colors.HexColor("#102A43"),
    )
    author_style = ParagraphStyle(
        "PaperAuthor",
        fontName=regular,
        fontSize=9.2,
        leading=11,
        alignment=TA_CENTER,
        spaceAfter=2,
    )
    abstract_heading = ParagraphStyle(
        "AbstractHeading",
        fontName=bold,
        fontSize=9.5,
        leading=11,
        alignment=TA_CENTER,
        spaceBefore=6,
        spaceAfter=4,
    )
    code_style = ParagraphStyle(
        "PaperCode",
        fontName=mono,
        fontSize=6.7,
        leading=8,
        leftIndent=4,
        rightIndent=4,
        borderColor=colors.HexColor("#CBD5E1"),
        borderWidth=0.4,
        borderPadding=4,
        backColor=colors.HexColor("#F8FAFC"),
        spaceBefore=3,
        spaceAfter=4,
    )
    list_style = ParagraphStyle(
        "PaperList",
        parent=body_style,
        firstLineIndent=0,
        leftIndent=0,
        spaceAfter=1.5,
    )
    table_style = ParagraphStyle(
        "PaperTable",
        fontName=regular,
        fontSize=6.8,
        leading=8.0,
        alignment=TA_LEFT,
    )
    table_head_style = ParagraphStyle(
        "PaperTableHead",
        parent=table_style,
        fontName=bold,
        textColor=colors.white,
    )

    title_h = 4.65 * inch
    lower_h = page_h - margin_top - margin_bottom - title_h - 0.10 * inch
    first_frames = [
        Frame(margin_x, page_h - margin_top - title_h, usable_w, title_h, id="title", showBoundary=0),
        Frame(margin_x, margin_bottom, col_w, lower_h, id="first-col-1", leftPadding=0, rightPadding=5),
        Frame(margin_x + col_w + gap, margin_bottom, col_w, lower_h, id="first-col-2", leftPadding=5, rightPadding=0),
    ]
    full_h = page_h - margin_top - margin_bottom
    two_frames = [
        Frame(margin_x, margin_bottom, col_w, full_h, id="col-1", leftPadding=0, rightPadding=5),
        Frame(margin_x + col_w + gap, margin_bottom, col_w, full_h, id="col-2", leftPadding=5, rightPadding=0),
    ]

    short_title = "Sandbox Insights"

    def first_page(canvas, doc):
        canvas.saveState()
        page_number = canvas.getPageNumber()
        if page_number > 1:
            canvas.setFont(regular, 7)
            canvas.setFillColor(colors.HexColor("#52667A"))
            canvas.drawString(margin_x, page_h - 0.33 * inch, short_title)
            canvas.drawRightString(page_w - margin_x, page_h - 0.33 * inch, "Capability-Directed Runtime Design")
            canvas.setStrokeColor(colors.HexColor("#CBD5E1"))
            canvas.setLineWidth(0.35)
            canvas.line(margin_x, page_h - 0.40 * inch, page_w - margin_x, page_h - 0.40 * inch)
        canvas.setStrokeColor(colors.HexColor("#9FB3C8"))
        canvas.setLineWidth(0.4)
        canvas.line(margin_x, 0.39 * inch, page_w - margin_x, 0.39 * inch)
        canvas.setFont(regular, 7)
        canvas.setFillColor(colors.HexColor("#52667A"))
        canvas.drawCentredString(page_w / 2, 0.22 * inch, str(page_number))
        canvas.restoreState()

    doc = PaperDocTemplate(
        str(output),
        metadata,
        pagesize=letter,
        leftMargin=margin_x,
        rightMargin=margin_x,
        topMargin=margin_top,
        bottomMargin=margin_bottom,
        pageTemplates=[
            PageTemplate(
                id="First",
                frames=first_frames,
                autoNextPageTemplate="Two",
            ),
            PageTemplate(
                id="Two",
                frames=two_frames,
                autoNextPageTemplate="Two",
            ),
        ],
        title=metadata["title"],
        author=metadata["author"],
    )
    doc.page_decorator = first_page

    story = [
        Paragraph(inline_pdf(metadata["title"], mono), title_style),
        Paragraph(inline_pdf(metadata["author"], mono), author_style),
        Paragraph(inline_pdf(metadata["date"], mono), author_style),
        HRFlowable(width="72%", thickness=0.6, color=colors.HexColor("#486581"), spaceBefore=5, spaceAfter=3),
        Paragraph("Abstract", abstract_heading),
    ]
    for block in abstract:
        if block.kind == "paragraph":
            story.append(Paragraph(inline_pdf(str(block.value), mono), abstract_style))
    keywords = ", ".join(metadata["keywords"])
    story.append(Paragraph(f"<b>Keywords:</b> {inline_pdf(keywords, mono)}", abstract_style))
    story.append(FrameBreak())

    heading_styles = {2: h1, 3: h2, 4: h3}
    for block in body:
        if block.kind == "heading":
            story.append(Paragraph(inline_pdf(str(block.value), mono), heading_styles.get(block.level, h3)))
        elif block.kind == "paragraph":
            story.append(Paragraph(inline_pdf(str(block.value), mono), body_style))
        elif block.kind == "code":
            _, code = block.value
            story.append(Preformatted(str(code), code_style, maxLineLength=58))
        elif block.kind == "list":
            list_type, items = block.value
            flow_items = [
                ListItem(Paragraph(inline_pdf(item, mono), list_style), leftIndent=10)
                for item in items
            ]
            story.append(
                ListFlowable(
                    flow_items,
                    bulletType="1" if list_type == "number" else "bullet",
                    start="1",
                    leftIndent=14,
                    bulletFontName=regular,
                    bulletFontSize=7.5,
                    spaceBefore=2,
                    spaceAfter=3,
                )
            )
        elif block.kind == "table":
            rows = block.value
            if not rows:
                continue
            ncols = max(len(row) for row in rows)
            data = []
            for row_idx, row in enumerate(rows):
                padded = row + [""] * (ncols - len(row))
                style = table_head_style if row_idx == 0 else table_style
                data.append([Paragraph(inline_pdf(cell, mono), style) for cell in padded])
            widths = [col_w / ncols] * ncols
            table = LongTable(data, colWidths=widths, repeatRows=1, hAlign="LEFT")
            table.setStyle(
                TableStyle(
                    [
                        ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#334E68")),
                        ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
                        ("VALIGN", (0, 0), (-1, -1), "TOP"),
                        ("GRID", (0, 0), (-1, -1), 0.25, colors.HexColor("#BCCCDC")),
                        ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, colors.HexColor("#F5F7FA")]),
                        ("LEFTPADDING", (0, 0), (-1, -1), 3),
                        ("RIGHTPADDING", (0, 0), (-1, -1), 3),
                        ("TOPPADDING", (0, 0), (-1, -1), 2.5),
                        ("BOTTOMPADDING", (0, 0), (-1, -1), 2.5),
                    ]
                )
            )
            story.append(KeepTogether([Spacer(1, 3), table, Spacer(1, 5)]))

    doc.build(story)


def verify_pdf(path: Path) -> tuple[int, int]:
    reader = PdfReader(str(path))
    pages = len(reader.pages)
    text = "\n".join((page.extract_text() or "") for page in reader.pages)
    phrases = [
        "seven-plane environment model",
        "Invalid-argument errno discrimination",
        "Observation without ptrace",
        "Unified architecture",
        "Evidence corpus and method",
        "residual risk",
    ]
    missing = [phrase for phrase in phrases if phrase.lower() not in text.lower()]
    if missing:
        raise ValueError(f"PDF text verification failed; missing: {missing}")
    if pages < 4:
        raise ValueError(f"PDF unexpectedly short: {pages} pages")
    return pages, len(text)


def main() -> int:
    script_dir = Path(__file__).resolve().parent
    project_root = script_dir.parent
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, default=project_root / "paper" / "paper.md")
    parser.add_argument("--metadata", type=Path, default=project_root / "paper" / "metadata.json")
    parser.add_argument("--pdf", type=Path, default=project_root / "paper" / "sandbox-insights.pdf")
    parser.add_argument("--tex", type=Path, default=project_root / "paper" / "main.tex")
    args = parser.parse_args()

    try:
        metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
        blocks = parse_markdown(args.input.read_text(encoding="utf-8"))
        abstract, body = split_abstract(blocks)
        args.pdf.parent.mkdir(parents=True, exist_ok=True)
        write_latex(metadata, abstract, body, args.tex)
        build_pdf(metadata, abstract, body, args.pdf)
        pages, chars = verify_pdf(args.pdf)
    except Exception as exc:
        print(f"FAIL paper build: {exc}", file=sys.stderr)
        return 1

    print(f"PASS latex={args.tex}")
    print(f"PASS pdf={args.pdf}")
    print(f"PASS pages={pages} extracted_chars={chars}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
