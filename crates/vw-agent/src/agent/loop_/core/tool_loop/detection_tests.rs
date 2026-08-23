//! 测试无工具调用却声称完成副作用时的保守检测规则。

use super::*;

#[test]
fn completion_detection_requires_completion_action_and_object() {
    assert!(looks_like_unverified_action_completion_without_tool_call("Done, I updated the file."));
    assert!(!looks_like_unverified_action_completion_without_tool_call("Done."));
    assert!(!looks_like_unverified_action_completion_without_tool_call("I will update the file."));
    assert!(!looks_like_unverified_action_completion_without_tool_call(""));
}

#[test]
fn completion_detection_trims_text_and_matches_case_insensitively() {
    assert!(looks_like_unverified_action_completion_without_tool_call(
        "\n  SUCCESSFULLY RAN THE COMMAND.  \t"
    ));
}

#[test]
fn completion_detection_accepts_supported_completion_cues() {
    let texts = [
        "I have created the directory.",
        "I've run the script.",
        "We've saved the files.",
        "We have renamed the folder.",
        "Finished moving the path.",
        "Completed, deleted the workspace.",
    ];

    for text in texts {
        assert!(looks_like_unverified_action_completion_without_tool_call(text), "{text}");
    }
}

#[test]
fn completion_detection_accepts_supported_side_effect_actions() {
    let texts = [
        "Done, create the file.",
        "Done, wrote the file.",
        "Done, ran the command.",
        "Done, executed the script.",
        "Done, removed the folder.",
        "Done, installed the files.",
        "Done, made the directory.",
    ];

    for text in texts {
        assert!(looks_like_unverified_action_completion_without_tool_call(text), "{text}");
    }
}

#[test]
fn completion_detection_accepts_supported_side_effect_objects() {
    let texts = [
        "Done, updated the folders.",
        "Done, updated the directories.",
        "Done, updated the cwd.",
        "Done, updated the current working directory.",
        "Done, updated the commands.",
        "Done, updated the scripts.",
        "Done, updated the paths.",
    ];

    for text in texts {
        assert!(looks_like_unverified_action_completion_without_tool_call(text), "{text}");
    }
}

#[test]
fn completion_detection_rejects_partial_word_matches() {
    let texts =
        ["Doneness created the file.", "Done, recreated the file.", "Done, updated the filename."];

    for text in texts {
        assert!(!looks_like_unverified_action_completion_without_tool_call(text), "{text}");
    }
}
