use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowCanvasContextMenuTarget {
    Canvas,
    Node(String),
    NodeInsert(String),
    Edge(String),
}

#[derive(Debug, Clone)]
pub struct WorkflowCanvasContextMenu {
    pub target: WorkflowCanvasContextMenuTarget,
    pub anchor: Point,
    pub world: Point,
}


pub fn open_context_menu(target: WorkflowCanvasContextMenuTarget, anchor: Point, world: Point)
    -> (WorkflowCanvasContextMenu, Option<String>, Option<String>)
{
    let (node, edge) = match &target {
        WorkflowCanvasContextMenuTarget::Canvas => (None, None),
        WorkflowCanvasContextMenuTarget::Node(id) | WorkflowCanvasContextMenuTarget::NodeInsert(id) => (Some(id.clone()), None),
        WorkflowCanvasContextMenuTarget::Edge(id) => (None, Some(id.clone())),
    };
    (WorkflowCanvasContextMenu { target, anchor, world }, node, edge)
}

pub fn context_menu_new_node_position(menu: Option<&WorkflowCanvasContextMenu>, document: &WorkflowDocument) -> Point {
    let Some(menu) = menu else { return Point::new(80.0, 80.0); };
    match &menu.target {
        WorkflowCanvasContextMenuTarget::Canvas | WorkflowCanvasContextMenuTarget::Edge(_) => menu.world,
        WorkflowCanvasContextMenuTarget::Node(id) | WorkflowCanvasContextMenuTarget::NodeInsert(id) => document.node(id)
            .map(|node| Point::new(node.position.x + node.size.width + 120.0, node.position.y + 18.0)).unwrap_or(menu.world),
    }
}

pub fn context_menu_auto_connect_source_node_id(menu: Option<&WorkflowCanvasContextMenu>) -> Option<String> {
    menu.and_then(|menu| match &menu.target {
        WorkflowCanvasContextMenuTarget::NodeInsert(id) => Some(id.clone()),
        _ => None,
    })
}

impl WorkflowAppEditorDraft {
    pub fn set_name(&mut self, value: String) { self.name = value; }
    pub fn set_description(&mut self, value: String) { self.description = value; }
    pub fn set_icon(&mut self, value: String) { self.icon = value; }
}
