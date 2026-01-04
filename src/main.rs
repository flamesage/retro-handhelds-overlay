#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

const APP_ICON_BYTES: &[u8] = include_bytes!("../icon.png");

const WINDOW_SIZE: [f32; 2] = [500.0, 120.0];

const PANEL_PADDING: f32 = 24.0;
const HORIZONTAL_GAP: f32 = 18.0;
const TEXT_VERTICAL_GAP: f32 = 6.0;

const ALPHA_THRESHOLD: u8 = 1;

const BG_RGBA: (u8, u8, u8, u8) = (30, 41, 59, 240);
const NAME_RGB: (u8, u8, u8) = (147, 197, 253);
const TITLE_RGB: (u8, u8, u8) = (226, 232, 240);
const DATE_RGB: (u8, u8, u8) = (148, 163, 184);

#[derive(Serialize, Deserialize, Clone)]
struct Settings {
    username: String,
    game_title: String,
}

impl Settings {
    fn file_path() -> PathBuf {
        let mut path =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        path.pop();
        path.push("settings.toml");
        path
    }

    fn load_or_create() -> Self {
        let path = Self::file_path();

        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(settings) = toml::from_str::<Settings>(&contents) {
                return settings;
            }
        }

        let defaults = Settings {
            username: "Your Name".to_string(),
            game_title: "Your Game".to_string(),
        };

        let _ = fs::write(
            &path,
            toml::to_string_pretty(&defaults).unwrap_or_default(),
        );

        defaults
    }
}

struct OverlayApp {
    icon_texture: Option<egui::TextureHandle>,
    date_string: String,
    settings: Settings,
}

impl Default for OverlayApp {
    fn default() -> Self {
        Self {
            icon_texture: None,
            date_string: Local::now().format("%B %-d, %Y").to_string(),
            settings: Settings::load_or_create(),
        }
    }
}

fn trim_transparent_pixels(
    rgba: &[u8],
    width: usize,
    height: usize,
    alpha_threshold: u8,
) -> (Vec<u8>, usize, usize) {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found_visible_pixel = false;

    for y in 0..height {
        for x in 0..width {
            let alpha = rgba[(y * width + x) * 4 + 3];
            if alpha > alpha_threshold {
                found_visible_pixel = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found_visible_pixel {
        return (rgba.to_vec(), width, height);
    }

    let new_width = max_x - min_x + 1;
    let new_height = max_y - min_y + 1;

    let mut output = vec![0u8; new_width * new_height * 4];

    for y in 0..new_height {
        for x in 0..new_width {
            let src = ((min_y + y) * width + (min_x + x)) * 4;
            let dst = (y * new_width + x) * 4;
            output[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }

    (output, new_width, new_height)
}

/// Draws a texture centered inside a fixed rectangular area,
/// scaling it to fit while preserving aspect ratio.
fn draw_icon_centered(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    icon_area_size: egui::Vec2,
) {
    let (icon_rect, _) =
        ui.allocate_exact_size(icon_area_size, egui::Sense::hover());

    let texture_size = texture.size_vec2();
    if texture_size.x <= 0.0 || texture_size.y <= 0.0 {
        return;
    }

    let scale = (icon_area_size.x / texture_size.x)
        .min(icon_area_size.y / texture_size.y);

    let draw_size = texture_size * scale;

    let draw_rect =
        egui::Rect::from_center_size(icon_rect.center(), draw_size);

    ui.painter().image(
        texture.id(),
        draw_rect,
        egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(1.0, 1.0),
        ),
        egui::Color32::WHITE,
    );
}

fn enable_window_drag(ui: &mut egui::Ui) {
    let drag_response = ui.interact(
        ui.max_rect(),
        ui.id().with("window_drag"),
        egui::Sense::drag(),
    );

    if drag_response.drag_started() {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

fn compute_icon_area(ui: &egui::Ui) -> egui::Vec2 {
    let available_height = ui.available_height().max(1.0);

    let height = (available_height * 0.70).clamp(36.0, 64.0);
    let width = (height * 4.0 / 3.0).clamp(48.0, 80.0);

    egui::vec2(width, height)
}

fn draw_text_block(
    ui: &mut egui::Ui,
    settings: &Settings,
    date: &str,
) {
    ui.spacing_mut().item_spacing.y = TEXT_VERTICAL_GAP;

    let name_color =
        egui::Color32::from_rgb(NAME_RGB.0, NAME_RGB.1, NAME_RGB.2);
    let title_color =
        egui::Color32::from_rgb(TITLE_RGB.0, TITLE_RGB.1, TITLE_RGB.2);
    let date_color =
        egui::Color32::from_rgb(DATE_RGB.0, DATE_RGB.1, DATE_RGB.2);

    let base_text_size = ui
        .style()
        .text_styles[&egui::TextStyle::Body]
        .size
        * 1.45;

    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(&settings.username)
                .strong()
                .size(base_text_size)
                .color(name_color),
        );

        ui.label(
            egui::RichText::new(&settings.game_title)
                .size(base_text_size)
                .color(title_color),
        );

        ui.label(
            egui::RichText::new(date)
                .size(base_text_size)
                .color(date_color),
        );
    });
}

impl eframe::App for OverlayApp {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        // Load and process icon once
        if self.icon_texture.is_none() {
            if let Ok(image) = image::load_from_memory(APP_ICON_BYTES) {
                let rgba = image.to_rgba8();
                let (pixels, w, h) = trim_transparent_pixels(
                    &rgba,
                    rgba.width() as usize,
                    rgba.height() as usize,
                    ALPHA_THRESHOLD,
                );

                let icon_image =
                    egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);

                self.icon_texture = Some(ctx.load_texture(
                    "overlay_icon",
                    icon_image,
                    Default::default(),
                ));
            }
        }

        let background_color = egui::Color32::from_rgba_premultiplied(
            BG_RGBA.0,
            BG_RGBA.1,
            BG_RGBA.2,
            BG_RGBA.3,
        );

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(background_color)
                    .inner_margin(egui::Margin::same(PANEL_PADDING)),
            )
            .show(ctx, |ui| {
                enable_window_drag(ui);

                let icon_area_size = compute_icon_area(ui);

                ui.spacing_mut().item_spacing.x = HORIZONTAL_GAP;

                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if let Some(icon) = &self.icon_texture {
                            draw_icon_centered(ui, icon, icon_area_size);
                        } else {
                            ui.allocate_exact_size(
                                icon_area_size,
                                egui::Sense::hover(),
                            );
                        }

                        ui.add_space(16.0);
                        draw_text_block(ui, &self.settings, &self.date_string);
                    },
                );
            });
    }
}

fn load_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(APP_ICON_BYTES)
        .ok()?
        .to_rgba8();

    Some(egui::IconData {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_SIZE)
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_icon(load_window_icon().unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "Retro Handhelds Overlay",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals {
                window_fill: egui::Color32::TRANSPARENT,
                panel_fill: egui::Color32::TRANSPARENT,
                ..Default::default()
            });

            Ok(Box::<OverlayApp>::default())
        }),
    )
}
