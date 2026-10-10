//! Gateway 路由覆盖门槛。

use std::collections::HashSet;

use clap::CommandFactory;

use super::coverage::{RouteCoverage, classify_route};

#[test]
fn api_route_coverage_classifies_every_documented_gateway_route() {
    let routes = vw_agent::gateway::documented_gateway_routes();
    let missing = routes
        .iter()
        .filter(|route| classify_route(route).is_none())
        .map(|route| format!("{} {}", route.method, route.path))
        .collect::<Vec<_>>();

    assert_eq!(routes.len(), 260, "Gateway route baseline changed");
    assert!(missing.is_empty(), "unclassified Gateway routes:\n{}", missing.join("\n"));

    let mut keys = HashSet::new();
    for route in &routes {
        assert!(
            keys.insert((route.method, route.path)),
            "duplicate route {} {}",
            route.method,
            route.path
        );
    }
    let command = crate::cli::Cli::command();
    let families = command.get_subcommands().map(clap::Command::get_name).collect::<HashSet<_>>();
    let mut command_count = 0;
    let mut alias_count = 0;
    let mut excluded_count = 0;
    for route in &routes {
        match classify_route(route).expect("classified") {
            RouteCoverage::Command(family) => {
                command_count += 1;
                assert!(
                    families.contains(family),
                    "route {} {} maps to missing api family {family}",
                    route.method,
                    route.path
                );
            }
            RouteCoverage::Alias(target) => {
                alias_count += 1;
                assert!(!target.trim().is_empty());
            }
            RouteCoverage::Excluded(target) => {
                excluded_count += 1;
                assert!(!target.trim().is_empty());
            }
        }
    }
    assert_eq!((command_count, alias_count, excluded_count), (247, 7, 6));
}
