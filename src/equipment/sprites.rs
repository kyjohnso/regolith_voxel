//! Equipment sprite resources and components

use bevy::prelude::*;
use std::collections::HashMap;

use super::EquipmentType;

/// Resource to store equipment sprite handles (normal and selected versions)
#[derive(Resource, Default)]
pub struct EquipmentSprites {
    pub normal: HashMap<EquipmentType, Handle<Image>>,
    pub selected: HashMap<EquipmentType, Handle<Image>>,
}

/// Component to mark equipment sprite entities
#[derive(Component)]
pub struct EquipmentSprite {
    pub equipment_id: usize,
    pub equipment_type: EquipmentType,
}