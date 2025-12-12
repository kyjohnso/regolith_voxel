//! Equipment module - handles mining equipment types, trees, sprites, and systems

mod types;
mod tree;
mod sprites;
mod systems;

pub use types::EquipmentType;
pub use tree::{
    EquipmentTreeNode, EquipmentTreeState, EquipmentTreeActions, SelectedEquipment
};
pub use sprites::{EquipmentSprites, EquipmentSprite, SelectionOutline};
pub use systems::{
    load_equipment_sprites,
    spawn_equipment_sprites,
    update_equipment_positions,
    click_select_equipment,
    move_selected_equipment,
    update_selection_outlines,
    equipment_mining_system,
};