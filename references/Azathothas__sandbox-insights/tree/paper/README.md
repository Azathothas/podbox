# Paper exports

`paper.md` is canonical and should be reviewed first. `main.tex` and
`sandbox-insights.pdf` are secondary generated exports.

From the repository root, rebuild and validate everything with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run-gate.ps1
```

The gate locates Python, generates a two-column PDF with ReportLab, emits LaTeX
for submission-oriented editing, validates PDF metadata and extracted text, and
renders every page for visual inspection. A TeX compiler is not required for the
PDF build.

Do not hand-edit `main.tex` or the PDF. Change `paper.md`, the metadata, or the
builder, then rebuild. Rendered PNGs under `paper/rendered/` are ignored QA files.
