//! UI system using egui

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_arbor::Outliner;

use crate::equipment::{
    EquipmentTreeActions, EquipmentTreeNode, EquipmentTreeState, EquipmentType, SelectedEquipment,
    AttachmentType, SpawnAttachmentEvent, MinerBody,
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
    attachment_query: Query<(&crate::equipment::AttachmentState, &crate::equipment::AttachmentSprite)>,
    miner_query: Query<(&crate::equipment::EquipmentSprite, &crate::equipment::MinerBody)>,
) {
    let ctx = contexts.ctx_mut();

    // Top panel
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Regolith Voxel - Mining Operations");
            ui.separator();
            ui.label("WASD: Pan | Q/E: Zoom | Click: Select | Arrows: Move | M: Mine | T: Attach | X: Detach");

            if !selected.selected_ids.is_empty() {
                ui.separator();
                if selected.selected_ids.len() == 1 {
                    ui.label(format!("Selected: Unit #{}", selected.selected_ids.iter().next().unwrap()));
                } else {
                    ui.label(format!("Selected: {} units", selected.selected_ids.len()));
                }
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

            // Show attachment options based on what's selected (BEFORE the tree)
            if !selected.selected_ids.is_empty() {
                // Check if any selected items are attached attachments
                let has_attached_attachments = selected.selected_ids.iter().any(|id| {
                    attachment_query
                        .iter()
                        .any(|(state, sprite)| {
                            sprite.attachment_id == *id
                                && matches!(*state, crate::equipment::AttachmentState::AttachedToEquipment { .. })
                        })
                });

                // Check if any selected items are detached attachments
                let has_detached_attachments = selected.selected_ids.iter().any(|id| {
                    attachment_query
                        .iter()
                        .any(|(state, sprite)| {
                            sprite.attachment_id == *id
                                && matches!(*state, crate::equipment::AttachmentState::Detached { .. })
                        })
                });

                if has_attached_attachments {
                    // Show detach option for attached attachments
                    ui.heading("Attachment Options");
                    if selected.selected_ids.len() == 1 {
                        ui.label("Press 'X' key to detach from parent");
                    } else {
                        ui.label(format!("Press 'X' key to detach {} attachments", selected.selected_ids.len()));
                    }
                    ui.add_space(5.0);
                }

                if has_detached_attachments {
                    // Show attach option for detached attachments
                    ui.heading("Attachment Options");
                    if selected.selected_ids.len() == 1 {
                        ui.label("Move near equipment and press 'T' key to attach (max 20 units)");
                    } else {
                        ui.label(format!("Move {} detached attachments near equipment and press 'T' to attach", selected.selected_ids.len()));
                    }
                    ui.add_space(5.0);
                }

                // Check if any selected items are equipment (not attachments)
                let selected_equipment: Vec<_> = selected.selected_ids
                    .iter()
                    .filter(|id| {
                        equipment_query.iter().any(|(_, sprite)| sprite.equipment_id == **id)
                    })
                    .copied()
                    .collect();

                if !selected_equipment.is_empty() {
                    // Show spawn options for equipment
                    ui.heading("Attachments");
                    if selected_equipment.len() == 1 {
                        ui.label(format!("Add to Unit #{}:", selected_equipment[0]));
                    } else {
                        ui.label(format!("Add to {} selected units:", selected_equipment.len()));
                    }
                    ui.add_space(5.0);

                    // Attachment buttons - will spawn on all selected equipment
                    ui.horizontal(|ui| {
                        if ui.button("+ Transmitter").clicked() {
                            // Spawn attachment on each selected equipment
                            for equipment_id in &selected_equipment {
                                let selected_pos = equipment_query
                                    .iter()
                                    .find_map(|(transform, sprite)| {
                                        if sprite.equipment_id == *equipment_id {
                                            Some(transform.translation.truncate())
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(Vec2::ZERO);

                                spawn_attachment_events.write(SpawnAttachmentEvent {
                                    attachment_type: AttachmentType::Transmitter,
                                    position: selected_pos,
                                    parent_equipment_id: Some(*equipment_id),
                                });
                            }
                        }
                        if ui.button("+ Receiver").clicked() {
                            for equipment_id in &selected_equipment {
                                let selected_pos = equipment_query
                                    .iter()
                                    .find_map(|(transform, sprite)| {
                                        if sprite.equipment_id == *equipment_id {
                                            Some(transform.translation.truncate())
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(Vec2::ZERO);

                                spawn_attachment_events.write(SpawnAttachmentEvent {
                                    attachment_type: AttachmentType::Receiver,
                                    position: selected_pos,
                                    parent_equipment_id: Some(*equipment_id),
                                });
                            }
                        }
                        if ui.button("+ Computer").clicked() {
                            for equipment_id in &selected_equipment {
                                let selected_pos = equipment_query
                                    .iter()
                                    .find_map(|(transform, sprite)| {
                                        if sprite.equipment_id == *equipment_id {
                                            Some(transform.translation.truncate())
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(Vec2::ZERO);

                                spawn_attachment_events.write(SpawnAttachmentEvent {
                                    attachment_type: AttachmentType::Computer,
                                    position: selected_pos,
                                    parent_equipment_id: Some(*equipment_id),
                                });
                            }
                        }
                    });
                }

                ui.add_space(10.0);
                ui.separator();
            }

            // Show inventory for selected mining equipment
            if !selected.selected_ids.is_empty() {
                // Find miners in the selection
                let selected_miners: Vec<_> = selected.selected_ids
                    .iter()
                    .filter_map(|id| {
                        miner_query.iter().find_map(|(sprite, miner_body)| {
                            if sprite.equipment_id == *id {
                                Some((*id, miner_body))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();

                if !selected_miners.is_empty() {
                    ui.heading("Inventory");
                    ui.separator();

                    for (equipment_id, miner_body) in selected_miners.iter() {
                        // Show equipment name and capacity
                        let capacity_percent = (miner_body.current_inventory_mass() / miner_body.capacity) * 100.0;
                        ui.label(format!(
                            "Unit #{} - {:.1}/{:.1} kg ({:.0}% full)",
                            equipment_id,
                            miner_body.current_inventory_mass(),
                            miner_body.capacity,
                            capacity_percent
                        ));

                        // Show capacity bar
                        let capacity_ratio = miner_body.current_inventory_mass() / miner_body.capacity;
                        let bar_color = if capacity_ratio > 0.9 {
                            egui::Color32::RED
                        } else if capacity_ratio > 0.7 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::GREEN
                        };

                        ui.add(
                            egui::ProgressBar::new(capacity_ratio)
                                .fill(bar_color)
                                .show_percentage()
                        );

                        // Show inventory contents
                        if !miner_body.inventory.is_empty() {
                            ui.label("Contents:");
                            for (mineral_type, amount) in miner_body.inventory.iter() {
                                ui.horizontal(|ui| {
                                    let color = mineral_type.color();
                                    ui.colored_label(
                                        egui::Color32::from_rgb(
                                            (color.to_srgba().red * 255.0) as u8,
                                            (color.to_srgba().green * 255.0) as u8,
                                            (color.to_srgba().blue * 255.0) as u8,
                                        ),
                                        "■"
                                    );
                                    ui.label(format!("{:?}: {:.1} kg", mineral_type, amount));
                                });
                            }
                        } else {
                            ui.label("Empty");
                        }

                        ui.add_space(5.0);
                    }

                    ui.add_space(5.0);
                    ui.separator();
                }
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