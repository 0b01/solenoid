use std::process::Command;

#[cfg(test)]
fn test_contract_factory(contract: &str, main_c: &str) -> String {
    Command::new("./target/debug/solenoid")
        .args(&["--input", contract, "-o", "./test_contract"])
        .spawn().unwrap().wait();

    Command::new("cp").args(&[main_c, "./test_contract/src/main.c"]).spawn().unwrap().wait();
    Command::new("./tests/build.sh")
        .arg("./test_contract/src/contracts.ll")
        .arg("./test_contract/src/main.c")
        .spawn().unwrap().wait();

    let output = Command::new("./bin/contracts.exe").output().unwrap();
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_contract_set() {
    let contract = "./tests/contracts/set.sol";
    let main_c = "./tests/main/main_set.c";

    let output = test_contract_factory(contract, main_c);
    assert_eq!("0000000000000000000000000000000000000000000000000000000000000001\n0000000000000000000000000000000000000000000000000000000000000005",
        output);
}

#[test]
fn test_contract_flipper() {
    let contract = "./tests/contracts/flipper.sol";
    let main_c = "./tests/main/main_flipper.c";

    let output = test_contract_factory(contract, main_c);

    let expected = r#"0000000000000000000000000000000000000000000000000000000000000001
0000000000000000000000000000000000000000000000000000000000000000
0000000000000000000000000000000000000000000000000000000000000001
0000000000000000000000000000000000000000000000000000000000000001
0000000000000000000000000000000000000000000000000000000000000001
0000000000000000000000000000000000000000000000000000000000000000
0000000000000000000000000000000000000000000000000000000000000000
0000000000000000000000000000000000000000000000000000000000000001
0000000000000000000000000000000000000000000000000000000000000000
0000000000000000000000000000000000000000000000000000000000000000
0000000000000000000000000000000000000000000000000000000000000000
0000000000000000000000000000000000000000000000000000000000000001
"#;

    assert_eq!(expected, output);
}

#[test]
fn test_contract_safemath() {
    let contract = "./tests/contracts/safemath.sol";
    let main_c = "./tests/main/main_safemath.c";

    let output = test_contract_factory(contract, main_c);

    let expected = r#"000000000000000000000000000000000000000000000000000000000000AAA9"#;

    assert_eq!(expected, output);
}