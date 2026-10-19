//! Dify 节点契约 fixture，冻结计划覆盖节点的最小字段与未知键保留行为。

use super::*;

struct DifyNodeFixture {
    block_type: &'static str,
    data_yaml: &'static str,
    required_fields: &'static [&'static str],
}

const DIFY_NODE_FIXTURES: [DifyNodeFixture; 19] = [
    DifyNodeFixture {
        block_type: "start",
        data_yaml: "variables: []",
        required_fields: &["variables"],
    },
    DifyNodeFixture {
        block_type: "llm",
        data_yaml: "model: { provider: openai, name: gpt, mode: chat }\nprompt_template: []\ncontext: { enabled: false, variable_selector: [] }\nvision: { enabled: false }",
        required_fields: &["model", "prompt_template", "context", "vision"],
    },
    DifyNodeFixture { block_type: "answer", data_yaml: "answer: ''", required_fields: &["answer"] },
    DifyNodeFixture { block_type: "end", data_yaml: "outputs: []", required_fields: &["outputs"] },
    DifyNodeFixture { block_type: "if-else", data_yaml: "cases: []", required_fields: &["cases"] },
    DifyNodeFixture {
        block_type: "code",
        data_yaml: "variables: []\ncode_language: python3\ncode: ''\noutputs: {}\nretry_config: { max_retries: 1, retry_interval: 100 }\nerror_strategy: fail-branch",
        required_fields: &[
            "variables",
            "code_language",
            "code",
            "outputs",
            "retry_config",
            "error_strategy",
        ],
    },
    DifyNodeFixture {
        block_type: "tool",
        data_yaml: "provider_id: builtin\nprovider_type: builtin\nprovider_name: builtin\ntool_name: tool\ntool_parameters: {}",
        required_fields: &[
            "provider_id",
            "provider_type",
            "provider_name",
            "tool_name",
            "tool_parameters",
        ],
    },
    DifyNodeFixture {
        block_type: "knowledge-retrieval",
        data_yaml: "query_variable_selector: [start, query]\ndataset_ids: []\nretrieval_mode: multiple",
        required_fields: &["query_variable_selector", "dataset_ids", "retrieval_mode"],
    },
    DifyNodeFixture {
        block_type: "question-classifier",
        data_yaml: "query_variable_selector: [start, query]\nmodel: { provider: openai, name: gpt, mode: chat }\nclasses: []\ninstruction: ''",
        required_fields: &["query_variable_selector", "model", "classes", "instruction"],
    },
    DifyNodeFixture {
        block_type: "parameter-extractor",
        data_yaml: "query: [start, query]\nmodel: { provider: openai, name: gpt, mode: chat }\nparameters: []\nreasoning_mode: prompt\ninstruction: ''",
        required_fields: &["query", "model", "parameters", "reasoning_mode", "instruction"],
    },
    DifyNodeFixture {
        block_type: "http-request",
        data_yaml: "method: get\nurl: https://example.com\nauthorization: { type: no-auth }\nheaders: ''\nparams: ''\nbody: { type: none, data: [] }\ntimeout: { connect: 10, read: 60, write: 20 }\nssl_verify: true\nretry_config: { max_retries: 1, retry_interval: 100 }\nerror_strategy: fail-branch",
        required_fields: &[
            "method",
            "url",
            "authorization",
            "headers",
            "params",
            "body",
            "timeout",
            "ssl_verify",
            "retry_config",
            "error_strategy",
        ],
    },
    DifyNodeFixture {
        block_type: "iteration",
        data_yaml: "iterator_selector: [start, items]\noutput_selector: [iteration, item]\noutput_type: array[string]\nis_parallel: false\nparallel_nums: 10\nerror_handle_mode: terminated",
        required_fields: &[
            "iterator_selector",
            "output_selector",
            "output_type",
            "is_parallel",
            "parallel_nums",
            "error_handle_mode",
        ],
    },
    DifyNodeFixture {
        block_type: "loop",
        data_yaml: "loop_variables: []\nbreak_conditions: []\nlogical_operator: and\nmax_loop_count: 10",
        required_fields: &[
            "loop_variables",
            "break_conditions",
            "logical_operator",
            "max_loop_count",
        ],
    },
    DifyNodeFixture {
        block_type: "variable-assigner",
        data_yaml: "version: '2'\nitems: []",
        required_fields: &["version", "items"],
    },
    DifyNodeFixture {
        block_type: "variable-aggregator",
        data_yaml: "variables: []\noutput_type: string",
        required_fields: &["variables", "output_type"],
    },
    DifyNodeFixture {
        block_type: "list-operator",
        data_yaml: "variable: [start, items]\nfilter_by: { enabled: false, conditions: [] }\norder_by: { enabled: false, key: '', value: asc }\nlimit: { enabled: false, size: 10 }",
        required_fields: &["variable", "filter_by", "order_by", "limit"],
    },
    DifyNodeFixture {
        block_type: "agent",
        data_yaml: "agent_strategy_provider_name: builtin\nagent_strategy_name: function_calling\nagent_parameters: {}\nmemory: { enabled: false }",
        required_fields: &[
            "agent_strategy_provider_name",
            "agent_strategy_name",
            "agent_parameters",
            "memory",
        ],
    },
    DifyNodeFixture {
        block_type: "template-transform",
        data_yaml: "variables: []\ntemplate: ''",
        required_fields: &["variables", "template"],
    },
    DifyNodeFixture {
        block_type: "document-extractor",
        data_yaml: "variable_selector: [start, file]",
        required_fields: &["variable_selector"],
    },
];

#[test]
fn dify_node_fixtures_preserve_unknown_fields() {
    for (index, fixture) in DIFY_NODE_FIXTURES.iter().enumerate() {
        let mut data: Value = serde_yaml::from_str(fixture.data_yaml).expect("fixture data YAML");
        let data_map = data.as_mapping_mut().expect("fixture data should be a map");
        data_map.insert("type".into(), fixture.block_type.into());
        data_map.insert("title".into(), format!("Fixture {index}").into());
        data_map.insert("desc".into(), "official-minimum".into());
        data_map.insert(
            "x-vibewindow-unknown".into(),
            yaml_map(vec![
                ("enabled", Value::Bool(true)),
                ("order", Value::Sequence(vec![index.into(), 99.into()])),
            ]),
        );
        let root = yaml_map(vec![(
            "workflow",
            yaml_map(vec![(
                "graph",
                yaml_map(vec![
                    (
                        "nodes",
                        Value::Sequence(vec![yaml_map(vec![
                            ("id", format!("node-{index}").into()),
                            ("type", "custom".into()),
                            ("data", data),
                        ])]),
                    ),
                    ("edges", Value::Sequence(Vec::new())),
                ]),
            )]),
        )]);

        let yaml = serde_yaml::to_string(&root).expect("fixture root YAML");
        let loaded = load_document_from_text(None, yaml).expect("fixture should load");
        let node = loaded.document.nodes.first().expect("fixture node");
        assert_eq!(node.block_type, fixture.block_type);

        let raw_data = node_data_yaml(node).expect("node data should serialize");
        let rebuilt = rebuild_node_from_parts(node, &node.title, &node.description, &raw_data)
            .expect("fixture should rebuild");
        let round_trip = node_data_yaml(&rebuilt).expect("rebuilt data should serialize");
        let value: Value = serde_yaml::from_str(&round_trip).expect("round-trip YAML");
        let map = value.as_mapping().expect("round-trip data map");
        for field in fixture.required_fields {
            assert!(map.contains_key(*field), "{} should keep {field}", fixture.block_type);
        }
        let unknown = map
            .get("x-vibewindow-unknown")
            .and_then(Value::as_mapping)
            .expect("unknown fixture field should remain");
        assert_eq!(unknown.get("enabled").and_then(Value::as_bool), Some(true));
        assert_eq!(unknown.get("order").and_then(Value::as_sequence).map(Vec::len), Some(2));
    }
}

#[test]
fn dify_node_fixtures_cover_registered_and_planned_types() {
    let fixture_types = DIFY_NODE_FIXTURES
        .iter()
        .map(|fixture| fixture.block_type)
        .collect::<std::collections::HashSet<_>>();
    let registered = supported_node_types()
        .iter()
        .map(|descriptor| descriptor.block_type)
        .collect::<std::collections::HashSet<_>>();

    assert!(registered.is_subset(&fixture_types));
    assert_eq!(fixture_types.len(), DIFY_NODE_FIXTURES.len());
}
