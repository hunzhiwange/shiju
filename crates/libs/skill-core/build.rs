use std::path::Path;

fn main() {
    let skills = "../../../skills/create-task-plan-skills";
    println!("cargo:rerun-if-changed={skills}");
    assert!(
        Path::new(skills).join("create-task-plans/SKILL.md").is_file(),
        "initialize the bundled skills with `git submodule update --init --recursive`"
    );
}
