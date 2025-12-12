//! Equipment sprite resources and components

use bevy::prelude::*;
use std::collections::HashMap;

use super::EquipmentType;

/// Resource to store equipment sprite handles
#[derive(Resource, Default)]
pub struct EquipmentSprites {
    pub sprites: HashMap<EquipmentType, Handle<Image>>,
}

/// Component to mark equipment sprite entities
#[derive(Component)]
pub struct EquipmentSprite {
    pub equipment_id: usize,
}

/// Component to mark selection outline sprites
#[derive(Component)]
pub struct SelectionOutline {
    pub equipment_id: usize,
}