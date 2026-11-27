use super::*;

#[test]
fn converting_and_copying_a_condition_preserves_its_rule() {
    let condition = WorkFilterNode::Condition(WorkFilterCondition {
        field: WorkFilterField::Label,
        excluded: true,
        rule: WorkViewFilter { label_ids: vec!["urgent".into()], ..Default::default() },
    });
    let mut draft = WorkAdvancedFilterDraft {
        simple: WorkViewFilter::default(),
        group: WorkFilterGroup { join: WorkFilterJoin::All, children: vec![condition.clone()] },
        editing: Some(vec![0]),
        menu: None,
    };

    convert_to_group(&mut draft, &[0]);
    duplicate_condition(&mut draft, &[0, 0]);
    assert_eq!(draft.editing, Some(vec![0, 0]));
    assert_eq!(draft.group.join, WorkFilterJoin::All);
    let WorkFilterNode::Group(inner) = &draft.group.children[0] else {
        panic!("condition should be in a group");
    };
    assert_eq!(inner.join, WorkFilterJoin::Any);
    assert_eq!(inner.children, vec![condition.clone(), condition]);
}

#[test]
fn inserting_and_removing_siblings_keep_the_active_editor_on_the_same_condition() {
    let mut draft = WorkAdvancedFilterDraft {
        simple: WorkViewFilter::default(),
        group: WorkFilterGroup {
            join: WorkFilterJoin::All,
            children: vec![
                new_condition(),
                WorkFilterNode::Group(WorkFilterGroup {
                    join: WorkFilterJoin::Any,
                    children: vec![new_condition(), new_condition()],
                }),
            ],
        },
        editing: Some(vec![1, 1]),
        menu: None,
    };

    duplicate_condition(&mut draft, &[0]);
    assert_eq!(draft.editing, Some(vec![2, 1]));
    remove_node(&mut draft, &[1]);
    assert_eq!(draft.editing, Some(vec![1, 1]));
    remove_node(&mut draft, &[1, 0]);
    assert_eq!(draft.editing, Some(vec![1, 0]));
    remove_node(&mut draft, &[1]);
    assert!(draft.editing.is_none());
}
