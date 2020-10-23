use std::process::Command;

#[test]
fn test_set() {

    Command::new("./target/debug/solenoid")
        .args(&["--input", "./tests/contracts/set.sol", "-o", "./test_set"])
        .spawn().unwrap().wait();

    Command::new("cp").args(&["./tests/main/main_set.c", "./test_set/src/main.c"]).spawn().unwrap().wait();
    Command::new("./tests/build.sh")
        .arg("./test_set/src/contracts.ll")
        .arg("./test_set/src/main.c")
        .spawn().unwrap().wait();

    let output = Command::new("./bin/contracts.exe").output().unwrap();
    assert_eq!("0000000000000000000000000000000000000000000000000000000000000001\n0000000000000000000000000000000000000000000000000000000000000005",
        &String::from_utf8_lossy(&output.stdout));
}