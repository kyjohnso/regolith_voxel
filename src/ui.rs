//! UI system using egui

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_arbor::Outliner;

use crate::equipment::{
    EquipmentTreeActions, EquipmentTreeNode, EquipmentTreeState, EquipmentType, SelectedEquipment,
    AttachmentType, SpawnAttachmentEvent,
};
use crate::scene::ViewMode;

/// Main UI system
pub fn ui_system(
    mut contexts: EguiContexts,
    mut equipment_state: ResMut<EquipmentTreeState>,
    mut equipment_actions: ResMut<EquipmentTreeActions>,
    selected: Res<SelectedEquipment>,
    mut view_mode: ResMut<ViewMode>,
    mut spawn_attachment_events: EventWriter<SpawnAttachmentEvent>,
    equipment_query: Query<(&Transform, &crate::equipment::EquipmentSprite)>,
) {
    let ctx = contexts.ctx_mut();

    // Top panel
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Regolith Voxel - Mining Operations");
            ui.separator();
            ui.label("WASD: Pan | Q/E: Zoom | Click: Select | Arrows: Move | M: Mine");

            if let Some(selected_id) = selected.selected_id {
                ui.separator();
                ui.label(format!("Selected: Unit #{}", selected_id));
            }
        });
    });

    // Bottom panel
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Status: Ready");
        });
    });

    // Left panel - Legend and View Toggle
    egui::SidePanel::left("left_panel").show(ctx, |ui| {
        ui.heading("View Mode");
        ui.separator();

        ui.label("Display:");
        if ui
            .selectable_label(*view_mode == ViewMode::Minerals, "Minerals")
            .clicked()
        {
            *view_mode = ViewMode::Minerals;
        }
        if ui
            .selectable_label(*view_mode == ViewMode::Heightmap, "Heightmap")
            .clicked()
        {
            *view_mode = ViewMode::Heightmap;
        }

        ui.add_space(10.0);
        ui.separator();

        if *view_mode == ViewMode::Minerals {
            ui.heading("Minerals");
            ui.separator();
            ui.label("Legend:");
            ui.colored_label(egui::Color32::from_rgb(204, 102, 51), "■ Iron");
            ui.colored_label(egui::Color32::from_rgb(184, 115, 51), "■ Copper");
            ui.colored_label(egui::Color32::from_rgb(255, 215, 0), "■ Gold");
            ui.colored_label(egui::Color32::from_rgb(192, 192, 192), "■ Silver");
            ui.colored_label(egui::Color32::from_rgb(51, 204, 51), "■ Uranium");
            ui.colored_label(egui::Color32::from_rgb(102, 204, 255), "■ Diamond");
            ui.colored_label(egui::Color32::from_rgb(51, 51, 51), "■ Coal");
        } else {
            ui.heading("Heightmap");
            ui.separator();
            ui.label("Height Legend:");
            ui.colored_label(egui::Color32::from_rgb(0, 0, 0), "■ 0 (Empty/Void)");
            ui.colored_label(egui::Color32::from_rgb(64, 64, 64), "■ Low");
            ui.colored_label(egui::Color32::from_rgb(128, 128, 128), "■ Medium");
            ui.colored_label(egui::Color32::from_rgb(192, 192, 192), "■ High");
            ui.colored_label(egui::Color32::from_rgb(255, 255, 255), "■ Very High");
        }
    });

    // Right panel - Equipment Tree with Outliner
    egui::SidePanel::right("right_panel")
        .min_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Mining Equipment");
            ui.separator();

            ui.label("Drag to reorganize | Double-click to rename");
            ui.add_space(4.0);

            // Action buttons at the top
            ui.horizontal(|ui| {
                if ui.button("+ New Container").clicked() {
                    let id = equipment_state.next_id;
                    equipment_state.add_container(format!("Container {}", id));
                }

                ui.menu_button("+ New Equipment", |ui| {
                    if ui.button("Sampler").clicked() {
                        let id = equipment_state.next_id;
                        equipment_state
                            .add_equipment(format!("Sampler {}", id), EquipmentType::Sampler);
                        ui.close_menu();
                    }
                    if ui.button("Surface Mining").clicked() {
                        let id = equipment_state.next_id;
                        equipment_state.add_equipment(
                            format!("Surface Miner {}", id),
                            EquipmentType::SurfaceMining,
                        );
                        ui.close_menu();
                    }
                    if ui.button("Deep Mining").clicked() {
                        let id = equipment_state.next_id;
                        equipment_state.add_equipment(
                            format!("Deep Miner {}", id),
                            EquipmentType::DeepMining,
                        );
                        ui.close_menu();
                    }
                    if ui.button("Refining").clicked() {
                        let id = equipment_state.next_id;
                        equipment_state
                            .add_equipment(format!("Refinery {}", id), EquipmentType::Refining);
                        ui.close_menu();
                    }
                    if ui.button("Transport").clicked() {
                        let id = equipment_state.next_id;
                        equipment_state
                            .add_equipment(format!("Transport {}", id), EquipmentType::Transport);
                        ui.close_menu();
                    }
                });
            });

            ui.separator();

            // Show attachment options when equipment is selected (BEFORE the tree)
            if let Some(selected_id) = selected.selected_id {
                ui.heading("Attachments");
                ui.label(format!("Add to Unit #{}:", selected_id));
                ui.add_space(5.0);

                // Find selected equipment position
                let selected_pos = equipment_query
                    .iter()
                    .find_map(|(transform, sprite)| {
                        if sprite.equipment_id == selected_id {
                            Some(transform.translation.truncate())
                        } else {
                            None
                        }
                    })
                    .unwrap_or(Vec2::ZERO);

                // Attachment buttons - system will handle standard offsets automatically
                ui.horizontal(|ui| {
                    if ui.button("+ Transmitter").clicked() {
                        spawn_attachment_events.write(SpawnAttachmentEvent {
                            attachment_type: AttachmentType::Transmitter,
                            position: selected_pos, // System adds standard offset
                            parent_equipment_id: Some(selected_id),
                        });
                    }
                    if ui.button("+ Receiver").clicked() {
                        spawn_attachment_events.write(SpawnAttachmentEvent {
                            attachment_type: AttachmentType::Receiver,
                            position: selected_pos, // System adds standard offset
                            parent_equipment_id: Some(selected_id),
                        });
                    }
                    if ui.button("+ Computer").clicked() {
                        spawn_attachment_events.write(SpawnAttachmentEvent {
                            attachment_type: AttachmentType::Computer,
                            position: selected_pos, // System adds standard offset
                            parent_equipment_id: Some(selected_id),
                        });
                    }
                });

                ui.add_space(10.0);
                ui.separator();
            }

            // Show the outliner with the tree
            egui::ScrollArea::vertical().show(ui, |ui| {
                let response = Outliner::new("equipment_outliner").show(
                    ui,
                    &equipment_state.nodes,
                    &mut *equipment_actions,
                );

                // Handle rename events
                if let Some((node_id, new_name)) = response.renamed() {
                    for root in &mut equipment_state.nodes {
                        if root.rename_node(*node_id, new_name.to_string()) {
                            break;
                        }
                    }
                }

                // Handle drag-drop events
                if let Some(drop_event) = response.drop_event() {
                    let target_id = &drop_event.target;
                    let position = drop_event.position;

                    // Get all nodes being dragged
                    let dragging_ids = response.dragging_nodes();

                    if !dragging_ids.is_empty() {
                        // Use TreeOperations to handle the move
                        for drag_id in dragging_ids {
                            // Find and remove the dragged node
                            let mut removed_node = None;

                            // Try to remove from root level
                            if let Some(idx) =
                                equipment_state.nodes.iter().position(|n| n.id == *drag_id)
                            {
                                removed_node = Some(equipment_state.nodes.remove(idx));
                            } else {
                                // Search recursively in children
                                for root in &mut equipment_state.nodes {
                                    if let Some(node) =
                                        EquipmentTreeNode::remove_node(root, *drag_id)
                                    {
                                        removed_node = Some(node);
                                        break;
                                    }
                                }
                            }

                            // Insert the node at the new position
                            if let Some(node) = removed_node {
                                let mut inserted = false;

                                // Try to insert relative to target
                                for root in &mut equipment_state.nodes {
                                    if EquipmentTreeNode::insert_node(
                                        root,
                                        *target_id,
                                        node.clone(),
                                        position,
                                    ) {
                                        inserted = true;
                                        break;
                                    }
                                }

                                // If not inserted, add back to root level
                                if !inserted {
                                    equipment_state.nodes.push(node);
                                }
                            }
                        }
                    }
                }
            });
        });

    // No central panel needed - game renders in the background
    // This allows clicks to reach the game without being intercepted by egui
}