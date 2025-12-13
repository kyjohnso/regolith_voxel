//! Equipment tree structure and state management

use bevy::prelude::*;
use egui_arbor::{ActionIcon, DropPosition, IconType, OutlinerActions, OutlinerNode, tree_ops::TreeOperations};
use std::collections::HashSet;

use super::EquipmentType;

/// Node type in the equipment tree
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType {
    Container,
    Equipment(EquipmentType),
}

/// Tree node for equipment hierarchy
#[derive(Debug, Clone)]
pub struct EquipmentTreeNode {
    pub id: usize,
    pub name: String,
    pub node_type: NodeType,
    pub position: Option<Vec2>,
    pub active: bool,
    pub children: Vec<EquipmentTreeNode>,
}

impl EquipmentTreeNode {
    /// Create a new container node
    pub fn container(id: usize, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            node_type: NodeType::Container,
            position: None,
            active: false,
            children: Vec::new(),
        }
    }

    /// Create a new equipment node
    pub fn equipment(id: usize, name: impl Into<String>, equipment_type: EquipmentType) -> Self {
        Self {
            id,
            name: name.into(),
            node_type: NodeType::Equipment(equipment_type),
            position: None,
            active: false,
            children: Vec::new(),
        }
    }

    /// Check if this node is a container
    pub fn is_container(&self) -> bool {
        matches!(self.node_type, NodeType::Container)
    }

    /// Get the equipment type if this is an equipment node
    pub fn equipment_type(&self) -> Option<EquipmentType> {
        match self.node_type {
            NodeType::Equipment(eq_type) => Some(eq_type),
            _ => None,
        }
    }

    /// Recursively find and rename a node by ID
    pub fn rename_node(&mut self, id: usize, new_name: String) -> bool {
        if self.id == id {
            self.name = new_name;
            return true;
        }

        for child in &mut self.children {
            if child.rename_node(id, new_name.clone()) {
                return true;
            }
        }

        false
    }

    /// Recursively find a node by ID and return a reference
    pub fn find_node(&self, id: usize) -> Option<&EquipmentTreeNode> {
        if self.id == id {
            return Some(self);
        }

        for child in &self.children {
            if let Some(node) = child.find_node(id) {
                return Some(node);
            }
        }

        None
    }

    /// Recursively find a mutable node by ID
    pub fn find_node_mut(&mut self, id: usize) -> Option<&mut EquipmentTreeNode> {
        if self.id == id {
            return Some(self);
        }

        for child in &mut self.children {
            if let Some(node) = child.find_node_mut(id) {
                return Some(node);
            }
        }

        None
    }

    /// Recursively collect all descendant IDs (children, grandchildren, etc.)
    pub fn collect_all_descendant_ids(&self, ids: &mut HashSet<usize>) {
        for child in &self.children {
            ids.insert(child.id);
            child.collect_all_descendant_ids(ids);
        }
    }

    /// Remove a node from the tree by ID
    pub fn remove_node(parent: &mut EquipmentTreeNode, id: usize) -> Option<EquipmentTreeNode> {
        // Check direct children
        if let Some(idx) = parent.children.iter().position(|n| n.id == id) {
            return Some(parent.children.remove(idx));
        }

        // Search recursively
        for child in &mut parent.children {
            if let Some(node) = Self::remove_node(child, id) {
                return Some(node);
            }
        }

        None
    }

    /// Insert a node at a position relative to a target
    pub fn insert_node(
        parent: &mut EquipmentTreeNode,
        target_id: usize,
        node: EquipmentTreeNode,
        position: DropPosition,
    ) -> bool {
        // If this is the target
        if parent.id == target_id {
            match position {
                DropPosition::Inside => {
                    if parent.is_container() {
                        parent.children.push(node);
                        return true;
                    }
                }
                _ => {
                    // Can't insert before/after root
                    return false;
                }
            }
        }

        // Check if target is in direct children
        if let Some(idx) = parent.children.iter().position(|n| n.id == target_id) {
            match position {
                DropPosition::Before => {
                    parent.children.insert(idx, node);
                    return true;
                }
                DropPosition::After => {
                    parent.children.insert(idx + 1, node);
                    return true;
                }
                DropPosition::Inside => {
                    if parent.children[idx].is_container() {
                        parent.children[idx].children.push(node);
                        return true;
                    }
                }
            }
        }

        // Search recursively
        for child in &mut parent.children {
            if Self::insert_node(child, target_id, node.clone(), position) {
                return true;
            }
        }

        false
    }
}

// Implement OutlinerNode for the tree
impl OutlinerNode for EquipmentTreeNode {
    type Id = usize;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_collection(&self) -> bool {
        self.is_container()
    }

    fn children(&self) -> &[Self] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<Self> {
        &mut self.children
    }

    fn icon(&self) -> Option<IconType> {
        if self.is_container() {
            Some(IconType::Collection)
        } else {
            Some(IconType::Entity)
        }
    }

    fn action_icons(&self) -> Vec<ActionIcon> {
        vec![ActionIcon::Visibility, ActionIcon::Selection]
    }
}

// Implement TreeOperations for drag-drop functionality
impl TreeOperations for EquipmentTreeNode {}

/// Resource to manage equipment tree state
#[derive(Resource)]
pub struct EquipmentTreeState {
    pub nodes: Vec<EquipmentTreeNode>,
    pub next_id: usize,
}

impl Default for EquipmentTreeState {
    fn default() -> Self {
        let mut next_id = 0;

        // Create initial container nodes for each equipment type with some sample equipment
        let nodes = vec![
            {
                let mut container = EquipmentTreeNode::container(next_id, "Samplers");
                next_id += 1;

                // Add a sample sampler
                container.children.push(EquipmentTreeNode::equipment(
                    next_id,
                    "Sampler Unit 1",
                    EquipmentType::Sampler,
                ));
                next_id += 1;

                container
            },
            {
                let mut container = EquipmentTreeNode::container(next_id, "Surface Mining");
                next_id += 1;

                // Add a sample surface miner
                container.children.push(EquipmentTreeNode::equipment(
                    next_id,
                    "Surface Miner 1",
                    EquipmentType::SurfaceMining,
                ));
                next_id += 1;

                container
            },
            {
                let mut container = EquipmentTreeNode::container(next_id, "Deep Mining");
                next_id += 1;

                // Add a sample deep miner
                container.children.push(EquipmentTreeNode::equipment(
                    next_id,
                    "Deep Miner 1",
                    EquipmentType::DeepMining,
                ));
                next_id += 1;

                container
            },
            {
                let container = EquipmentTreeNode::container(next_id, "Refining");
                next_id += 1;
                container
            },
            {
                let container = EquipmentTreeNode::container(next_id, "Transport");
                next_id += 1;
                container
            },
        ];

        Self { nodes, next_id }
    }
}

impl EquipmentTreeState {
    /// Add a new container to the tree
    pub fn add_container(&mut self, name: String) {
        let container = EquipmentTreeNode::container(self.next_id, name);
        self.next_id += 1;
        self.nodes.push(container);
    }

    /// Add a new equipment node to the tree
    pub fn add_equipment(&mut self, name: String, equipment_type: EquipmentType) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let equipment = EquipmentTreeNode::equipment(id, name, equipment_type);
        self.nodes.push(equipment);

        id
    }

    /// Find a node by ID
    pub fn find_node(&self, id: usize) -> Option<&EquipmentTreeNode> {
        for node in &self.nodes {
            if let Some(found) = node.find_node(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a mutable node by ID
    pub fn find_node_mut(&mut self, id: usize) -> Option<&mut EquipmentTreeNode> {
        for node in &mut self.nodes {
            if let Some(found) = node.find_node_mut(id) {
                return Some(found);
            }
        }
        None
    }
}

/// Actions handler for the outliner
#[derive(Resource, Default)]
pub struct EquipmentTreeActions {
    pub selected: HashSet<usize>,
    pub visible: HashSet<usize>,
}

impl EquipmentTreeActions {
    pub fn new() -> Self {
        Self {
            selected: HashSet::new(),
            visible: HashSet::new(),
        }
    }
}

impl OutlinerActions<EquipmentTreeNode> for EquipmentTreeActions {
    fn on_rename(&mut self, _id: &usize, _new_name: String) {
        // Renaming is handled in the ui_system
    }

    fn on_move(&mut self, _id: &usize, _target: &usize, _position: DropPosition) {
        // Moving is handled in the ui_system
    }

    fn on_select(&mut self, id: &usize, selected: bool) {
        if selected {
            self.selected.insert(*id);
        } else {
            self.selected.remove(id);
        }
    }

    fn is_selected(&self, id: &usize) -> bool {
        self.selected.contains(id)
    }

    fn is_visible(&self, id: &usize) -> bool {
        !self.visible.contains(id) // Using "visible" set as "hidden" set - inverted logic
    }

    fn is_locked(&self, _id: &usize) -> bool {
        false
    }

    fn on_visibility_toggle(&mut self, id: &usize) {
        if self.visible.contains(id) {
            self.visible.remove(id);
        } else {
            self.visible.insert(*id);
        }
    }

    fn on_lock_toggle(&mut self, _id: &usize) {}

    fn on_selection_toggle(&mut self, id: &usize) {
        let is_selected = self.is_selected(id);
        self.on_select(id, !is_selected);
    }

    fn on_custom_action(&mut self, _id: &usize, _icon: &str) {}
}

/// Resource to track selected equipment (supports multi-selection)
#[derive(Resource, Default)]
pub struct SelectedEquipment {
    pub selected_ids: HashSet<usize>,
}

/// Resource to manage attachment entities
#[derive(Resource)]
pub struct AttachmentManager {
    pub attachments: Vec<AttachmentInfo>,
    pub next_id: usize,
}

impl Default for AttachmentManager {
    fn default() -> Self {
        Self {
            attachments: Vec::new(),
            next_id: 1000, // Start at 1000 to avoid conflicts with equipment IDs
        }
    }
}

/// Information about a spawned attachment
#[derive(Debug, Clone)]
pub struct AttachmentInfo {
    pub id: usize,
    pub attachment_type: crate::equipment::AttachmentType,
    pub parent_equipment_id: Option<usize>, // None if detached
    pub offset_from_parent: bevy::prelude::Vec2, // Offset from parent equipment position
}

/// Resource to track mount point occupancy
#[derive(Resource, Default)]
pub struct MountPointOccupancy {
    /// Maps equipment_id -> mount slots
    pub mounts: std::collections::HashMap<usize, MountState>,
}

#[derive(Debug, Clone, Default)]
pub struct MountState {
    pub communication_mounts: [Option<usize>; 2], // 2 slots for transmitter/receiver
    pub control_mounts: [Option<usize>; 2],        // 2 slots for computer
}

impl MountPointOccupancy {
    /// Check if a mount point is available on equipment
    pub fn is_mount_available(&self, equipment_id: usize, mount_type: MountType) -> bool {
        if let Some(mount_state) = self.mounts.get(&equipment_id) {
            match mount_type {
                MountType::Communication => mount_state.communication_mounts.iter().any(|m| m.is_none()),
                MountType::Control => mount_state.control_mounts.iter().any(|m| m.is_none()),
            }
        } else {
            true // No entry means all mounts are empty
        }
    }

    /// Occupy a mount point - finds first available slot
    pub fn occupy_mount(&mut self, equipment_id: usize, mount_type: MountType, attachment_id: usize) -> bool {
        let mount_state = self.mounts.entry(equipment_id).or_default();
        match mount_type {
            MountType::Communication => {
                if let Some(slot) = mount_state.communication_mounts.iter_mut().find(|m| m.is_none()) {
                    *slot = Some(attachment_id);
                    true
                } else {
                    false
                }
            }
            MountType::Control => {
                if let Some(slot) = mount_state.control_mounts.iter_mut().find(|m| m.is_none()) {
                    *slot = Some(attachment_id);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Free a specific attachment from its mount
    pub fn free_attachment(&mut self, equipment_id: usize, attachment_id: usize) {
        if let Some(mount_state) = self.mounts.get_mut(&equipment_id) {
            // Check communication mounts
            for slot in &mut mount_state.communication_mounts {
                if *slot == Some(attachment_id) {
                    *slot = None;
                    return;
                }
            }
            // Check control mounts
            for slot in &mut mount_state.control_mounts {
                if *slot == Some(attachment_id) {
                    *slot = None;
                    return;
                }
            }
        }
    }

    /// Get count of occupied mounts for a type
    pub fn occupied_count(&self, equipment_id: usize, mount_type: MountType) -> usize {
        if let Some(mount_state) = self.mounts.get(&equipment_id) {
            match mount_type {
                MountType::Communication => mount_state.communication_mounts.iter().filter(|m| m.is_some()).count(),
                MountType::Control => mount_state.control_mounts.iter().filter(|m| m.is_some()).count(),
            }
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountType {
    Communication, // For transmitter/receiver
    Control,       // For computer
}

/// Event to request spawning an attachment
#[derive(bevy::prelude::Event)]
pub struct SpawnAttachmentEvent {
    pub attachment_type: crate::equipment::AttachmentType,
    pub position: bevy::prelude::Vec2,
    pub parent_equipment_id: Option<usize>,
}