//! Equipment sprite resources and components

use bevy::prelude::*;
use std::collections::HashMap;

use super::EquipmentType;

use super::AttachmentType;

/// Resource to store equipment sprite handles (normal and selected versions)
#[derive(Resource, Default)]
pub struct EquipmentSprites {
    pub normal: HashMap<EquipmentType, Handle<Image>>,
    pub selected: HashMap<EquipmentType, Handle<Image>>,
}

/// Resource to store attachment sprite handles
#[derive(Resource, Default)]
pub struct AttachmentSprites {
    pub normal: HashMap<AttachmentType, Handle<Image>>,
    pub selected: HashMap<AttachmentType, Handle<Image>>,
}

/// Component to mark equipment sprite entities
#[derive(Component)]
pub struct EquipmentSprite {
    pub equipment_id: usize,
    pub equipment_type: EquipmentType,
}

/// Component to mark attachment sprite entities
#[derive(Component)]
pub struct AttachmentSprite {
    pub attachment_id: usize,
    pub attachment_type: AttachmentType,
}