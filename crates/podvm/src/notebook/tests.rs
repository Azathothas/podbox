use super::cells::{outcome, segments};
use super::*;
use serde_json::json;

fn v(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}

#[test]
fn the_arguments_parse_and_a_wrong_one_is_named() {
    let a = parse(&v(&[
        "nb.ipynb",
        "--persist",
        "keep",
        "--cycles=3",
        "--kill-after",
        "5",
        "--restart-kernel-after",
        "1",
        "--restart-kernel-after=3",
    ]))
    .unwrap();
    assert_eq!(a.nb, PathBuf::from("nb.ipynb"));
    assert_eq!(a.persist, PathBuf::from("keep"));
    assert_eq!(a.cycles, 3);
    assert_eq!(a.kill_after, Some(5));
    assert_eq!(a.restart_after, vec![1, 3]);
    assert_eq!(
        parse(&v(&["nb.ipynb", "--persist", "k"])).unwrap().cycles,
        1
    );
    assert!(parse(&v(&["nb.ipynb"])).unwrap_err().contains("--persist"));
    assert!(parse(&v(&["--persist", "k"])).unwrap_err().contains("NB"));
    assert!(parse(&v(&["nb", "--persist", "k", "--cycles", "0"]))
        .unwrap_err()
        .contains("--cycles"));
    assert!(parse(&v(&["nb", "--persist", "k", "--lab"]))
        .unwrap_err()
        .contains("--lab"));
    assert!(parse(&v(&["a", "b", "--persist", "k"]))
        .unwrap_err()
        .contains("second"));
}

#[test]
fn each_output_is_a_line_of_its_cycle_and_an_error_raises() {
    let nb = json!({"cells": [
        {"cell_type": "code", "outputs": [
            {"output_type": "stream", "name": "stdout", "text": ["hello 42\n", "two\n"]},
            {"output_type": "execute_result", "data": {"text/plain": ["7"]}}
        ]},
        {"cell_type": "markdown"},
        {"cell_type": "code", "outputs": [
            {"output_type": "error", "ename": "ZeroDivisionError", "evalue": "division by zero"}
        ]}
    ]});
    let o = outcome(&nb, 2);
    assert_eq!(
        o.lines,
        v(&[
            "[2] hello 42",
            "[2] two",
            "[2] 7",
            "[2] ZeroDivisionError: division by zero"
        ])
    );
    assert!(o.raised);
    // A failed `!` line is a stream: no raise.
    let nb = json!({"cells": [{"outputs": [
        {"output_type": "stream", "name": "stderr", "text": "/bin/sh: 1: curl: not found\n"}
    ]}]});
    let o = outcome(&nb, 1);
    assert_eq!(o.lines, v(&["[1] /bin/sh: 1: curl: not found"]));
    assert!(!o.raised);
}

#[test]
fn a_restart_cuts_the_notebook_after_its_cell() {
    let nb = json!({"cells": [{"n": 1}, {"n": 2}, {"n": 3}], "nbformat": 4});
    let s = segments(&nb, &[1]).unwrap();
    assert_eq!(s.len(), 2);
    assert_eq!(s[0]["cells"], json!([{"n": 1}]));
    assert_eq!(s[1]["cells"], json!([{"n": 2}, {"n": 3}]));
    assert_eq!(s[1]["nbformat"], 4);
    // A cut at or past the last cell, and a repeated cut, cut nothing more.
    assert_eq!(segments(&nb, &[3, 9]).unwrap().len(), 1);
    assert_eq!(segments(&nb, &[2, 2, 1]).unwrap().len(), 3);
    assert!(segments(&json!({}), &[]).is_err());
}

#[test]
fn each_kernel_runs_with_a_read_only_root_and_only_persist_kept() {
    let a = run_argv(
        Path::new("/w"),
        Path::new("/keep"),
        JUPYTER_IMAGE,
        "nb.ipynb",
        false,
        Some(5),
    );
    for f in [
        "--rm",
        "--read-only",
        "--tmpfs=/tmp",
        "/w:/workspace",
        "/keep:/persist",
    ] {
        assert!(a.contains(&f.to_string()), "{f} missing: {a:?}");
    }
    assert!(a.contains(&"--allow-errors".to_string()));
    assert!(a.iter().any(|x| x.starts_with("--timeout=")));
    let m = run_argv(
        Path::new("/w"),
        Path::new("/k"),
        MARIMO_IMAGE,
        "a.py",
        true,
        None,
    );
    assert!(m.windows(2).any(|w| w == ["export", "html"]));
    assert!(!m.iter().any(|x| x.starts_with("--timeout=")));
}
