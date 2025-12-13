//! Equipment module - handles mining equipment types, trees, sprites, and systems

mod types;
mod tree;
mod sprites;
mod systems;

pub use types::{
    EquipmentType, EquipmentPiece, AttachmentState, MinerBody, SamplerBody, SmelterBody, TransportBody,
    Transmitter, Receiver, Computer, AttachmentType
};
pub use tree::{
    EquipmentTreeNode, EquipmentTreeState, EquipmentTreeActions, SelectedEquipment,
    AttachmentManager, AttachmentInfo, SpawnAttachmentEvent
};
pub use sprites::{EquipmentSprites, EquipmentSprite, AttachmentSprites, AttachmentSprite};
pub use systems::{
    load_equipment_sprites,
    load_attachment_sprites,
    spawn_equipment_sprites,
    spawn_attachments,
    update_equipment_positions,
    update_attachment_positions,
    click_select_equipment,
    move_selected_equipment,
    update_selection_sprites,
    delete_selected_attachment,
    equipment_mining_system,
};