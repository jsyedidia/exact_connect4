// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::process::Command;

#[test]
fn generator_depth_outputs_match_expected_values() {
    assert_eq!(run_generator_depth("0"), "\n");
    assert_eq!(run_generator_depth("1"), "\n1\n2\n3\n4\n");
    assert_eq!(
        run_generator_depth("2"),
        "\n1\n11\n12\n13\n14\n15\n16\n17\n2\n21\n22\n23\n24\n25\n26\n27\n3\n31\n32\n33\n34\n35\n36\n37\n4\n41\n42\n43\n44\n"
    );
}

fn run_generator_depth(depth: &str) -> String {
    let output = Command::new(generator_bin())
        .arg(depth)
        .output()
        .expect("failed to run generator");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
    String::from_utf8(output.stdout).unwrap()
}

fn generator_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_generator"))
}
