mod model;
mod report;
mod solver;

use std::path::PathBuf;

use eframe::egui;
use egui::{
    pos2, vec2, Align, Align2, Color32, FontId, Layout, Margin, RichText, Sense, Stroke, Vec2,
};

use model::{
    demo_project, Bearing, BearingKind, Gauge, Interaction, Load, LoadKind, Material, Member,
    PaperSize, Project, ReportFormat, UnitSystem,
};
use solver::{Results, Sample};

const INK: Color32 = Color32::from_rgb(31, 42, 52);
const MUTED: Color32 = Color32::from_rgb(105, 118, 131);
const BORDER: Color32 = Color32::from_rgb(214, 221, 229);
const CARD: Color32 = Color32::WHITE;
const BG: Color32 = Color32::from_rgb(241, 244, 248);
const ACCENT: Color32 = Color32::from_rgb(28, 91, 126);
const ACCENT_SOFT: Color32 = Color32::from_rgb(224, 236, 243);
const OK: Color32 = Color32::from_rgb(31, 122, 77);
const WARN: Color32 = Color32::from_rgb(181, 122, 18);
const DANGER: Color32 = Color32::from_rgb(176, 58, 46);
const LOAD_RED: Color32 = Color32::from_rgb(179, 65, 45);
const LOAD_AMBER: Color32 = Color32::from_rgb(184, 134, 43);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Model,
    Diagrams,
    Report,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Model => "Model & diagrams",
            Tab::Diagrams => "Result diagrams",
            Tab::Report => "Report studio",
        }
    }
}

fn solve_now(project: &Project) -> (Option<Results>, Option<String>) {
    match solver::solve(project) {
        Ok(r) => (Some(r), None),
        Err(e) => (None, Some(e)),
    }
}

fn setup_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = CARD;
    style.visuals.extreme_bg_color = Color32::WHITE;
    style.visuals.faint_bg_color = Color32::from_rgb(246, 248, 250);
    style.visuals.override_text_color = Some(INK);
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(6.0);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(250, 251, 252);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(246, 248, 250);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(10.0, 5.0);
    ctx.set_style(style);
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(10.0)
        .inner_margin(Margin::same(12.0))
        .show(ui, add)
        .inner
}

fn card_titled<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    card(ui, |ui| {
        ui.label(RichText::new(title).size(13.0).strong().color(INK));
        ui.add_space(6.0);
        add(ui)
    })
}

fn chip(ui: &mut egui::Ui, label: &str, value: impl Into<String>) {
    egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(6.0)
        .inner_margin(Margin::symmetric(9.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).size(9.5).color(MUTED));
                ui.label(RichText::new(value.into()).size(12.0).strong().color(INK));
            });
        });
}

fn text_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(RichText::new(label).size(10.5).color(MUTED));
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
    ui.add_space(3.0);
}

fn tiny_label(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).size(10.0).color(MUTED));
}

struct TipBeamApp {
    project: Project,
    results: Option<Results>,
    error: Option<String>,
    tab: Tab,
    sel_member: usize,
    sel_bearing: usize,
    sel_load: usize,
    status: String,
    project_path: String,
    about_open: bool,
    screenshot_path: Option<PathBuf>,
    screenshot_frames: u32,
    screenshot_requested: bool,
}

impl TipBeamApp {
    fn new(cc: &eframe::CreationContext<'_>, screenshot_path: Option<PathBuf>) -> Self {
        setup_style(&cc.egui_ctx);
        let project = demo_project();
        let (results, error) = solve_now(&project);
        let status = if screenshot_path.is_some() {
            "Rendering application view...".to_string()
        } else {
            "Demo project loaded. Edit the model on the left, then export or print the report.".to_string()
        };
        Self {
            project,
            results,
            error,
            tab: Tab::Model,
            sel_member: 0,
            sel_bearing: 0,
            sel_load: 0,
            status,
            project_path: "beam-project.json".to_string(),
            about_open: false,
            screenshot_path,
            screenshot_frames: 0,
            screenshot_requested: false,
        }
    }

    fn clamp_selections(&mut self) {
        let members = self.project.members.len();
        if members == 0 {
            self.sel_member = 0;
        } else if self.sel_member >= members {
            self.sel_member = members - 1;
        }
        if self.project.bearings.is_empty() {
            self.sel_bearing = 0;
        } else if self.sel_bearing >= self.project.bearings.len() {
            self.sel_bearing = self.project.bearings.len() - 1;
        }
        if self.project.loads.is_empty() {
            self.sel_load = 0;
        } else if self.sel_load >= self.project.loads.len() {
            self.sel_load = self.project.loads.len() - 1;
        }
    }

    fn new_demo(&mut self) {
        self.project = demo_project();
        self.sel_member = 0;
        self.sel_bearing = 0;
        self.sel_load = 0;
        self.status = "Demo project loaded.".to_string();
    }

    fn save_project(&mut self) {
        let path = PathBuf::from(self.project_path.trim());
        if path.as_os_str().is_empty() {
            self.status = "Enter a project file name first.".to_string();
            return;
        }
        let result = serde_json::to_string_pretty(&self.project)
            .map_err(|e| e.to_string())
            .and_then(|text| std::fs::write(&path, text).map_err(|e| e.to_string()));
        self.status = match result {
            Ok(()) => format!("Project saved to {}", path.display()),
            Err(e) => format!("Could not save the project: {e}"),
        };
    }

    fn load_project(&mut self) {
        let path = PathBuf::from(self.project_path.trim());
        let result = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str::<Project>(&text).map_err(|e| e.to_string()));
        match result {
            Ok(project) => {
                self.project = project;
                self.clamp_selections();
                self.status = format!("Project loaded from {}", path.display());
            }
            Err(e) => self.status = format!("Could not load the project: {e}"),
        }
    }

    fn export_report(&mut self) {
        if self.results.is_none() {
            let (results, error) = solve_now(&self.project);
            self.results = results;
            self.error = error;
        }
        let outcome = match &self.results {
            Some(results) => report::write_report(&self.project, results),
            None => Err(self
                .error
                .clone()
                .unwrap_or_else(|| "No calculation results are available.".to_string())),
        };
        self.status = match outcome {
            Ok(path) => format!("Report exported to {}", path.display()),
            Err(e) => format!("Export failed: {e}"),
        };
    }

    fn print_report(&mut self) {
        if self.results.is_none() {
            let (results, error) = solve_now(&self.project);
            self.results = results;
            self.error = error;
        }
        let outcome = match &self.results {
            Some(results) => report::print_report(&self.project, results),
            None => Err(self
                .error
                .clone()
                .unwrap_or_else(|| "No calculation results are available.".to_string())),
        };
        self.status = match outcome {
            Ok(path) => format!("Print view written to {} and opened in the browser.", path.display()),
            Err(e) => format!("Could not open the print view: {e}"),
        };
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar")
            .frame(
                egui::Frame::none()
                    .fill(CARD)
                    .stroke(Stroke::new(1.0, BORDER))
                    .inner_margin(Margin::symmetric(12.0, 6.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New demo project").clicked() {
                            self.new_demo();
                            ui.close_menu();
                        }
                        if ui.button("Open project (JSON)").clicked() {
                            self.load_project();
                            ui.close_menu();
                        }
                        if ui.button("Save project (JSON)").clicked() {
                            self.save_project();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export report").clicked() {
                            self.export_report();
                            ui.close_menu();
                        }
                        if ui.button("Print report").clicked() {
                            self.print_report();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("View", |ui| {
                        if ui.button("Model & diagrams").clicked() {
                            self.tab = Tab::Model;
                            ui.close_menu();
                        }
                        if ui.button("Result diagrams").clicked() {
                            self.tab = Tab::Diagrams;
                            ui.close_menu();
                        }
                        if ui.button("Report studio").clicked() {
                            self.tab = Tab::Report;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Help", |ui| {
                        if ui.button("About this rebuild").clicked() {
                            self.about_open = true;
                            ui.close_menu();
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("Build {}", model::APP_VERSION))
                                .size(9.5)
                                .color(MUTED),
                        );
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 9.0, ACCENT);
                    painter.line_segment(
                        [
                            pos2(rect.left() + 7.0, rect.center().y),
                            pos2(rect.right() - 7.0, rect.center().y),
                        ],
                        Stroke::new(3.0, Color32::WHITE),
                    );
                    painter.circle_filled(
                        pos2(rect.left() + 8.0, rect.center().y + 7.0),
                        2.2,
                        Color32::WHITE,
                    );
                    painter.circle_filled(
                        pos2(rect.right() - 8.0, rect.center().y + 7.0),
                        2.2,
                        Color32::WHITE,
                    );
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(model::APP_NAME)
                                .size(17.0)
                                .strong()
                                .color(INK),
                        );
                        ui.label(
                            RichText::new("Continuous beam calculation and customizable reports")
                                .size(10.0)
                                .color(MUTED),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let print = ui.add(
                            egui::Button::new(RichText::new("Print").color(Color32::WHITE))
                                .fill(ACCENT),
                        );
                        if print.clicked() {
                            self.print_report();
                        }
                        if ui.button("Export report").clicked() {
                            self.export_report();
                        }
                        if ui.button("Calculate").clicked() {
                            let (results, error) = solve_now(&self.project);
                            self.results = results;
                            self.error = error;
                            self.status = "Beam recalculated.".to_string();
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for tab in [Tab::Model, Tab::Diagrams, Tab::Report] {
                        if ui
                            .selectable_label(
                                self.tab == tab,
                                RichText::new(tab.label()).size(12.0).strong(),
                            )
                            .clicked()
                        {
                            self.tab = tab;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let state = if self.error.is_some() {
                            RichText::new("Input needs attention").color(DANGER)
                        } else {
                            RichText::new("Live calculation").color(OK)
                        };
                        ui.label(state.size(10.0));
                    });
                });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        let message = if let Some(err) = &self.error {
            (err.clone(), DANGER)
        } else {
            (self.status.clone(), MUTED)
        };
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::none()
                    .fill(CARD)
                    .stroke(Stroke::new(1.0, BORDER))
                    .inner_margin(Margin::symmetric(12.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(message.0).size(10.5).color(message.1));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Euler-Bernoulli FEM · Eurocode partial factors")
                                .size(9.5)
                                .color(MUTED),
                        );
                    });
                });
            });
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(342.0)
            .width_range(300.0..=480.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(246, 248, 251))
                    .stroke(Stroke::new(1.0, BORDER))
                    .inner_margin(Margin::symmetric(10.0, 8.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_source("left_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.project_section(ui);
                        ui.add_space(8.0);
                        self.members_section(ui);
                        ui.add_space(8.0);
                        self.bearings_section(ui);
                        ui.add_space(8.0);
                        self.loads_section(ui);
                        ui.add_space(8.0);
                        self.calculation_section(ui);
                        ui.add_space(8.0);
                        self.files_section(ui);
                        ui.add_space(12.0);
                    });
            });
    }

    fn project_section(&mut self, ui: &mut egui::Ui) {
        card_titled(ui, "Project data", |ui| {
            text_field(ui, "Title", &mut self.project.title);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() * 0.5 - 4.0);
                    text_field(ui, "Project number", &mut self.project.project_no);
                });
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    text_field(ui, "Engineer", &mut self.project.engineer);
                });
            });
            text_field(ui, "Customer", &mut self.project.customer);
            ui.label(RichText::new("Comment").size(10.5).color(MUTED));
            ui.add(
                egui::TextEdit::multiline(&mut self.project.comment)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
        });
    }

    fn members_section(&mut self, ui: &mut egui::Ui) {
        card_titled(ui, "Members", |ui| {
            let count = self.project.members.len();
            egui::ScrollArea::vertical()
                .id_source("member_list")
                .max_height(128.0)
                .show(ui, |ui| {
                    for i in 0..count {
                        let m = &self.project.members[i];
                        let text = format!(
                            "{:>2}   {:<18} {:>7.0} mm",
                            i + 1,
                            truncate(&m.type_name, 18),
                            m.length_mm
                        );
                        if ui
                            .selectable_label(self.sel_member == i, RichText::new(text).size(10.5))
                            .clicked()
                        {
                            self.sel_member = i;
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.small_button("Add").clicked() {
                    let mut member = Member::default();
                    member.length_mm = 4000.0;
                    self.project.members.push(member);
                    self.sel_member = self.project.members.len() - 1;
                }
                if ui
                    .add_enabled(count > 1, egui::Button::new("Remove").small())
                    .clicked()
                {
                    let idx = self.sel_member.min(self.project.members.len() - 1);
                    self.project.members.remove(idx);
                    self.clamp_selections();
                    self.status = "Member removed.".to_string();
                }
                if ui.add_enabled(count > 0, egui::Button::new("Duplicate").small()).clicked() {
                    let idx = self.sel_member.min(self.project.members.len() - 1);
                    let member = self.project.members[idx].clone();
                    self.project.members.insert(idx + 1, member);
                    self.sel_member = idx + 1;
                }
            });
            ui.separator();
            self.member_editor(ui);
        });
    }

    fn member_editor(&mut self, ui: &mut egui::Ui) {
        if self.project.members.is_empty() {
            ui.label(RichText::new("Add a member to start.").color(MUTED));
            return;
        }
        let idx = self.sel_member.min(self.project.members.len() - 1);
        self.sel_member = idx;
        let member = &mut self.project.members[idx];
        ui.label(
            RichText::new(format!("Member {} · properties", idx + 1))
                .size(11.0)
                .color(ACCENT),
        );
        ui.add_space(4.0);
        egui::Grid::new("member_grid")
            .num_columns(2)
            .spacing([10.0, 5.0])
            .show(ui, |ui| {
                ui.label("Section name");
                ui.add(
                    egui::TextEdit::singleline(&mut member.type_name).desired_width(150.0),
                );
                ui.end_row();

                ui.label("Section library");
                let mut gauge = member.gauge.clone();
                egui::ComboBox::from_id_source("member_gauge")
                    .selected_text(gauge.label())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for option in [
                            Gauge::TimberI20,
                            Gauge::Timber,
                            Gauge::Steel,
                            Gauge::Aluminium,
                            Gauge::Custom,
                        ] {
                            ui.selectable_value(&mut gauge, option.clone(), option.label());
                        }
                    });
                if gauge != member.gauge {
                    member.apply_gauge(gauge);
                }
                ui.end_row();

                ui.label("Material");
                let mut material = member.material.clone();
                egui::ComboBox::from_id_source("member_material")
                    .selected_text(material.label())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for option in [
                            Material::Timber,
                            Material::Steel,
                            Material::Aluminium,
                            Material::Other,
                        ] {
                            ui.selectable_value(&mut material, option.clone(), option.label());
                        }
                    });
                if material != member.material {
                    member.material = material;
                }
                ui.end_row();

                ui.label("Length");
                ui.add(
                    egui::DragValue::new(&mut member.length_mm)
                        .clamp_range(100.0..=60000.0)
                        .speed(10.0)
                        .suffix(" mm"),
                );
                ui.end_row();

                ui.label("Inertia I");
                ui.add(
                    egui::DragValue::new(&mut member.i_cm4)
                        .clamp_range(0.1..=10_000_000.0)
                        .speed(10.0)
                        .suffix(" cm4"),
                );
                ui.end_row();

                ui.label("E module");
                ui.add(
                    egui::DragValue::new(&mut member.e_kn_cm2)
                        .clamp_range(0.1..=100_000.0)
                        .speed(1.0)
                        .suffix(" kN/cm2"),
                );
                ui.end_row();

                ui.label("Permissible M");
                ui.add(
                    egui::DragValue::new(&mut member.perm_m_knm)
                        .clamp_range(0.0..=100_000.0)
                        .speed(0.01)
                        .suffix(" kNm"),
                );
                ui.end_row();

                ui.label("Permissible V");
                ui.add(
                    egui::DragValue::new(&mut member.perm_v_kn)
                        .clamp_range(0.0..=100_000.0)
                        .speed(0.05)
                        .suffix(" kN"),
                );
                ui.end_row();

                ui.label("Parts");
                ui.add(egui::DragValue::new(&mut member.parts).clamp_range(1..=99));
                ui.end_row();

                ui.label("Self weight");
                ui.add(
                    egui::DragValue::new(&mut member.self_weight_kn_m)
                        .clamp_range(0.0..=100.0)
                        .speed(0.01)
                        .suffix(" kN/m"),
                );
                ui.end_row();
            });
        tiny_label(
            ui,
            format!(
                "EI = {:.1} kNm2 · {:.2} m long",
                member.ei_knm2(),
                member.length_m()
            ),
        );
    }

    fn bearings_section(&mut self, ui: &mut egui::Ui) {
        card_titled(ui, "Bearings", |ui| {
            let count = self.project.bearings.len();
            egui::ScrollArea::vertical()
                .id_source("bearing_list")
                .max_height(104.0)
                .show(ui, |ui| {
                    for i in 0..count {
                        let b = &self.project.bearings[i];
                        let text = format!(
                            "Joint {:>2}   {:<22}",
                            b.node + 1,
                            truncate(&b.kind.label(), 22)
                        );
                        if ui
                            .selectable_label(self.sel_bearing == i, RichText::new(text).size(10.5))
                            .clicked()
                        {
                            self.sel_bearing = i;
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.small_button("Add").clicked() {
                    let node = self.sel_bearing.min(self.project.node_count().saturating_sub(1));
                    self.project.bearings.push(Bearing {
                        node,
                        ..Default::default()
                    });
                    self.sel_bearing = self.project.bearings.len() - 1;
                }
                if ui
                    .add_enabled(count > 0, egui::Button::new("Remove").small())
                    .clicked()
                {
                    let idx = self.sel_bearing.min(self.project.bearings.len() - 1);
                    self.project.bearings.remove(idx);
                    self.clamp_selections();
                    self.status = "Bearing removed.".to_string();
                }
            });
            ui.separator();
            self.bearing_editor(ui);
        });
    }

    fn bearing_editor(&mut self, ui: &mut egui::Ui) {
        if self.project.bearings.is_empty() {
            ui.label(RichText::new("No bearing defined.").color(MUTED));
            return;
        }
        let idx = self.sel_bearing.min(self.project.bearings.len() - 1);
        self.sel_bearing = idx;
        let nodes = self.project.node_count();
        let bnode = self.project.bearings[idx].node;
        let x_mm = self.project.node_x_mm(bnode);
        let bearing = &mut self.project.bearings[idx];
        ui.label(
            RichText::new(format!("Bearing {} · condition", idx + 1))
                .size(11.0)
                .color(ACCENT),
        );
        ui.add_space(4.0);
        egui::Grid::new("bearing_grid")
            .num_columns(2)
            .spacing([10.0, 5.0])
            .show(ui, |ui| {
                ui.label("Joint");
                let mut joint = bearing.node + 1;
                if ui
                    .add(egui::DragValue::new(&mut joint).clamp_range(1..=nodes))
                    .changed()
                {
                    bearing.node = joint.saturating_sub(1).min(nodes.saturating_sub(1));
                }
                ui.end_row();

                ui.label("Condition");
                let mut kind = bearing.kind.clone();
                egui::ComboBox::from_id_source("bearing_kind")
                    .selected_text(kind.label())
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for option in BearingKind::all() {
                            ui.selectable_value(&mut kind, option.clone(), option.label());
                        }
                    });
                if kind != bearing.kind {
                    bearing.kind = kind;
                }
                ui.end_row();

                ui.label("Horizontal restraint");
                ui.checkbox(&mut bearing.horizontal, "");
                ui.end_row();

                ui.label("Moment hinge");
                ui.checkbox(&mut bearing.moment_hinge, "");
                ui.end_row();

                if matches!(bearing.kind, BearingKind::Elastic) {
                    ui.label("Spring k");
                    ui.add(
                        egui::DragValue::new(&mut bearing.spring_kn_mm)
                            .clamp_range(0.0..=1_000_000.0)
                            .speed(0.1)
                            .suffix(" kN/mm"),
                    );
                    ui.end_row();

                    ui.label("Support area A");
                    ui.add(
                        egui::DragValue::new(&mut bearing.section_mm2)
                            .clamp_range(0.0..=10_000_000.0)
                            .speed(100.0)
                            .suffix(" mm2"),
                    );
                    ui.end_row();

                    ui.label("Support E");
                    ui.add(
                        egui::DragValue::new(&mut bearing.e_kn_cm2)
                            .clamp_range(0.0..=100_000.0)
                            .speed(1.0)
                            .suffix(" kN/cm2"),
                    );
                    ui.end_row();

                    ui.label("Support height");
                    ui.add(
                        egui::DragValue::new(&mut bearing.length_mm)
                            .clamp_range(0.0..=100_000.0)
                            .speed(10.0)
                            .suffix(" mm"),
                    );
                    ui.end_row();
                }
            });
        ui.label(RichText::new("Comment").size(10.5).color(MUTED));
        ui.add(
            egui::TextEdit::singleline(&mut bearing.comment)
                .desired_width(f32::INFINITY)
                .hint_text("optional note"),
        );
        tiny_label(
            ui,
            format!(
                "x = {:.0} mm · k = {:.2} kN/mm",
                x_mm,
                bearing.spring_kn_mm()
            ),
        );
    }

    fn loads_section(&mut self, ui: &mut egui::Ui) {
        card_titled(ui, "Loadings", |ui| {
            let count = self.project.loads.len();
            egui::ScrollArea::vertical()
                .id_source("load_list")
                .max_height(112.0)
                .show(ui, |ui| {
                    for i in 0..count {
                        let l = &self.project.loads[i];
                        let text = format!(
                            "{} {:>2}  {:<16} {}",
                            if l.active { "on " } else { "off" },
                            i + 1,
                            truncate(l.kind.label(), 16),
                            truncate(&l.summary(), 38)
                        );
                        if ui
                            .selectable_label(self.sel_load == i, RichText::new(text).size(9.7))
                            .clicked()
                        {
                            self.sel_load = i;
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.small_button("Add").clicked() {
                    self.project.loads.push(Load::default());
                    self.sel_load = self.project.loads.len() - 1;
                }
                if ui
                    .add_enabled(count > 0, egui::Button::new("Remove").small())
                    .clicked()
                {
                    let idx = self.sel_load.min(self.project.loads.len() - 1);
                    self.project.loads.remove(idx);
                    self.clamp_selections();
                    self.status = "Loading removed.".to_string();
                }
            });
            ui.separator();
            self.load_editor(ui);
        });
    }

    fn load_editor(&mut self, ui: &mut egui::Ui) {
        if self.project.loads.is_empty() {
            ui.label(RichText::new("No loading defined.").color(MUTED));
            return;
        }
        let idx = self.sel_load.min(self.project.loads.len() - 1);
        self.sel_load = idx;
        let total = self.project.total_length_mm();
        let load = &mut self.project.loads[idx];
        ui.label(
            RichText::new(format!("Loading {} · values", idx + 1))
                .size(11.0)
                .color(ACCENT),
        );
        ui.add_space(4.0);
        egui::Grid::new("load_grid")
            .num_columns(2)
            .spacing([10.0, 5.0])
            .show(ui, |ui| {
                ui.label("Type");
                let mut kind = load.kind.clone();
                egui::ComboBox::from_id_source("load_kind")
                    .selected_text(kind.label())
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for option in LoadKind::all() {
                            ui.selectable_value(&mut kind, option.clone(), option.label());
                        }
                    });
                if kind != load.kind {
                    load.kind = kind;
                }
                ui.end_row();

                ui.label("Active");
                ui.checkbox(&mut load.active, "");
                ui.end_row();

                match load.kind {
                    LoadKind::Point => {
                        ui.label("Force P");
                        ui.add(
                            egui::DragValue::new(&mut load.b)
                                .clamp_range(-10_000.0..=10_000.0)
                                .speed(0.1)
                                .suffix(" kN"),
                        );
                        ui.end_row();
                    }
                    LoadKind::Moment => {
                        ui.label("Moment M");
                        ui.add(
                            egui::DragValue::new(&mut load.a)
                                .clamp_range(-10_000.0..=10_000.0)
                                .speed(0.1)
                                .suffix(" kNm"),
                        );
                        ui.end_row();
                    }
                    LoadKind::Trapezoid => {
                        ui.label("Start q");
                        ui.add(
                            egui::DragValue::new(&mut load.a)
                                .clamp_range(-1000.0..=1000.0)
                                .speed(0.05)
                                .suffix(" kN/m"),
                        );
                        ui.end_row();
                        ui.label("End q");
                        ui.add(
                            egui::DragValue::new(&mut load.b)
                                .clamp_range(-1000.0..=1000.0)
                                .speed(0.05)
                                .suffix(" kN/m"),
                        );
                        ui.end_row();
                    }
                    LoadKind::ConcretePressure => {
                        ui.label("Start p");
                        ui.add(
                            egui::DragValue::new(&mut load.a)
                                .clamp_range(-1000.0..=1000.0)
                                .speed(0.05)
                                .suffix(" kN/m2"),
                        );
                        ui.end_row();
                        ui.label("End p");
                        ui.add(
                            egui::DragValue::new(&mut load.b)
                                .clamp_range(-1000.0..=1000.0)
                                .speed(0.05)
                                .suffix(" kN/m2"),
                        );
                        ui.end_row();
                        ui.label("Width");
                        ui.add(
                            egui::DragValue::new(&mut load.e)
                                .clamp_range(0.0..=20_000.0)
                                .speed(5.0)
                                .suffix(" mm"),
                        );
                        ui.end_row();
                    }
                }

                if matches!(
                    load.kind,
                    LoadKind::Point | LoadKind::Moment | LoadKind::Trapezoid | LoadKind::ConcretePressure
                ) {
                    ui.label("Start x");
                    ui.add(
                        egui::DragValue::new(&mut load.c)
                            .clamp_range(0.0..=total.max(1.0))
                            .speed(10.0)
                            .suffix(" mm"),
                    );
                    ui.end_row();
                }

                if matches!(
                    load.kind,
                    LoadKind::Trapezoid | LoadKind::ConcretePressure
                ) {
                    ui.label("Loaded length");
                    ui.add(
                        egui::DragValue::new(&mut load.d)
                            .clamp_range(0.0..=total.max(1.0))
                            .speed(10.0)
                            .suffix(" mm"),
                    );
                    ui.end_row();
                }
            });
        ui.horizontal(|ui| {
            if matches!(
                load.kind,
                LoadKind::Trapezoid | LoadKind::ConcretePressure
            ) && ui.small_button("Over whole beam").clicked()
            {
                load.c = 0.0;
                load.d = total;
            }
            if ui.small_button("Clear values").clicked() {
                load.a = 0.0;
                load.b = 0.0;
                load.c = 0.0;
                load.d = 0.0;
                load.e = 0.0;
            }
        });
        ui.label(RichText::new("Comment").size(10.5).color(MUTED));
        ui.add(
            egui::TextEdit::singleline(&mut load.comment)
                .desired_width(f32::INFINITY)
                .hint_text("optional note"),
        );
    }

    fn calculation_section(&mut self, ui: &mut egui::Ui) {
        card_titled(ui, "Calculation settings", |ui| {
            ui.checkbox(
                &mut self.project.calc.eurocode,
                "Eurocode partial factors",
            );
            ui.checkbox(
                &mut self.project.calc.include_self_weight,
                "Include member self weight",
            );
            ui.checkbox(
                &mut self.project.calc.actual_load_gamma,
                "Use gamma_F = 1.0 (actual loads)",
            );
            ui.add_space(3.0);
            egui::Grid::new("calc_grid")
                .num_columns(2)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    ui.label("gamma_F (load, 1.5-fold evidenced)");
                    ui.add(
                        egui::DragValue::new(&mut self.project.calc.gamma_f)
                            .clamp_range(1.0..=2.0)
                            .speed(0.01),
                    );
                    ui.end_row();
                    ui.label("gamma_M steel");
                    ui.add(
                        egui::DragValue::new(&mut self.project.calc.gamma_m_steel)
                            .clamp_range(1.0..=2.0)
                            .speed(0.01),
                    );
                    ui.end_row();
                    ui.label("gamma_M timber");
                    ui.add(
                        egui::DragValue::new(&mut self.project.calc.gamma_m_timber)
                            .clamp_range(1.0..=2.0)
                            .speed(0.01),
                    );
                    ui.end_row();
                    ui.label("k_mod timber");
                    ui.add(
                        egui::DragValue::new(&mut self.project.calc.kmod_timber)
                            .clamp_range(0.1..=1.0)
                            .speed(0.01),
                    );
                    ui.end_row();
                    ui.label("M-V interaction");
                    egui::ComboBox::from_id_source("interaction")
                        .selected_text(self.project.calc.interaction.label())
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for option in Interaction::all() {
                                ui.selectable_value(
                                    &mut self.project.calc.interaction,
                                    option.clone(),
                                    option.label(),
                                );
                            }
                        });
                    ui.end_row();
                });
        });
    }

    fn files_section(&mut self, ui: &mut egui::Ui) {
        card_titled(ui, "Project file and output", |ui| {
            text_field(ui, "Project file (JSON)", &mut self.project_path);
            ui.horizontal(|ui| {
                if ui.small_button("Save project").clicked() {
                    self.save_project();
                }
                if ui.small_button("Load project").clicked() {
                    self.load_project();
                }
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "Report: {}",
                    report::output_path(&self.project).display()
                ))
                .size(9.7)
                .color(MUTED),
            );
            ui.horizontal(|ui| {
                if ui.small_button("Export report").clicked() {
                    self.export_report();
                }
                if ui.small_button("Print report").clicked() {
                    self.print_report();
                }
            });
        });
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("results_panel")
            .resizable(true)
            .default_width(304.0)
            .width_range(260.0..=430.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(246, 248, 251))
                    .stroke(Stroke::new(1.0, BORDER))
                    .inner_margin(Margin::symmetric(10.0, 8.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_source("results_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Results").size(16.0).strong().color(INK));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new("live").size(9.5).color(OK));
                            });
                        });
                        ui.add_space(6.0);
                        if let Some(err) = &self.error {
                            egui::Frame::none()
                                .fill(Color32::from_rgb(253, 238, 236))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(232, 190, 184)))
                                .rounding(8.0)
                                .inner_margin(Margin::same(10.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("Calculation stopped")
                                            .strong()
                                            .color(DANGER),
                                    );
                                    ui.label(RichText::new(err).size(10.5).color(DANGER));
                                });
                        } else if let Some(results) = &self.results {
                            self.results_view(ui, results);
                        } else {
                            ui.label(RichText::new("No results yet.").color(MUTED));
                        }
                    });
            });
    }

    fn results_view(&self, ui: &mut egui::Ui, results: &Results) {
        let util = results.max_util;
        let (util_color, util_text) = if results.overloaded {
            (DANGER, "Overloaded")
        } else if util > 0.85 {
            (WARN, "Near limit")
        } else {
            (OK, "Within capacity")
        };
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("MAXIMUM UTILIZATION")
                            .size(9.5)
                            .color(MUTED),
                    );
                    ui.label(
                        RichText::new(format!("{:.2}", util))
                            .size(30.0)
                            .strong()
                            .color(util_color),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(util_text).strong().color(util_color));
                        ui.label(
                            RichText::new(format!("Member {}", results.max_util_member + 1))
                                .size(9.5)
                                .color(MUTED),
                        );
                    });
                });
            });
            ui.add(
                egui::ProgressBar::new((util.clamp(0.0, 1.0)) as f32)
                    .desired_width(f32::INFINITY)
                    .fill(util_color)
                    .text(format!("{:.0}%", util * 100.0)),
            );
        });
        ui.add_space(8.0);

        card_titled(ui, "Key values", |ui| {
            egui::Grid::new("key_values")
                .num_columns(2)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    key_row(ui, "Total length", format!("{:.2} m", self.project.total_length_m()));
                    key_row(
                        ui,
                        "Total vertical load",
                        format!("{:.2} kN", results.total_load_kn),
                    );
                    key_row(
                        ui,
                        "Sum of reactions",
                        format!("{:.2} kN", results.sum_reactions_kn),
                    );
                    key_row(ui, "Maximum |M|", format!("{:.2} kNm", results.max_m));
                    key_row(ui, "Maximum |V|", format!("{:.2} kN", results.max_v));
                    key_row(
                        ui,
                        "Maximum deflection",
                        format!("{:.2} mm", results.max_defl),
                    );
                    key_row(
                        ui,
                        "Maximum rotation",
                        format!("{:.5} rad", results.max_rot),
                    );
                    key_row(
                        ui,
                        "Load balance",
                        format!(
                            "{:.1} %",
                            100.0 * results.sum_reactions_kn / results.total_load_kn.max(1e-9)
                        ),
                    );
                });
        });
        ui.add_space(8.0);

        card_titled(ui, "Reactions", |ui| {
            for reaction in &results.reactions {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Joint {}", reaction.node + 1))
                            .size(10.5)
                            .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{:.2} kN", reaction.force_kn))
                                .size(10.5)
                                .strong()
                                .color(INK),
                        );
                    });
                });
                tiny_label(
                    ui,
                    format!(
                        "x = {:.0} mm · M = {:.2} kNm · {}",
                        reaction.x_mm, reaction.moment_knm, reaction.kind
                    ),
                );
                ui.add_space(2.0);
            }
        });
        ui.add_space(8.0);

        card_titled(ui, "Member checks", |ui| {
            for member in &results.members {
                let color = if member.overloaded {
                    DANGER
                } else if member.util > 0.85 {
                    WARN
                } else {
                    OK
                };
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Member {}", member.index + 1))
                            .size(10.5)
                            .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{:.2}", member.util))
                                .size(10.5)
                                .strong()
                                .color(color),
                        );
                    });
                });
                ui.add(
                    egui::ProgressBar::new(member.util.clamp(0.0, 1.0) as f32)
                        .desired_width(f32::INFINITY)
                        .fill(color),
                );
                tiny_label(
                    ui,
                    format!(
                        "M {:.2} / {:.2} kNm · V {:.2} / {:.2} kN",
                        member.m_max.abs().max(member.m_min.abs()),
                        member
                            .util_m
                            .partial_cmp(&0.0)
                            .map(|_| self.project.members[member.index].perm_m_knm)
                            .unwrap_or(0.0),
                        member.v_max.abs().max(member.v_min.abs()),
                        self.project.members[member.index].perm_v_kn
                    ),
                );
                ui.add_space(3.0);
            }
        });
        ui.add_space(8.0);

        card_titled(ui, "Model summary", |ui| {
            key_row(ui, "Members", format!("{}", self.project.members.len()));
            key_row(ui, "Joints", format!("{}", self.project.node_count()));
            key_row(ui, "Bearings", format!("{}", self.project.bearings.len()));
            key_row(ui, "Loadings", format!("{}", self.project.loads.len()));
            key_row(
                ui,
                "Active loads",
                format!(
                    "{}",
                    self.project.loads.iter().filter(|l| l.active).count()
                ),
            );
        });
        let issues = self.project.validate();
        if !issues.is_empty() {
            ui.add_space(8.0);
            card_titled(ui, "Input checks", |ui| {
                for issue in issues {
                    ui.label(RichText::new(format!("• {issue}")).size(10.0).color(WARN));
                }
            });
        }
    }

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                match self.tab {
                    Tab::Model => self.model_tab(ui),
                    Tab::Diagrams => self.diagrams_tab(ui),
                    Tab::Report => self.report_tab(ui),
                }
            });
    }

    fn model_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_source("model_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    chip(ui, "SPANS", format!("{}", self.project.members.len()));
                    chip(
                        ui,
                        "TOTAL LENGTH",
                        format!("{:.2} m", self.project.total_length_m()),
                    );
                    if let Some(results) = &self.results {
                        chip(
                            ui,
                            "TOTAL LOAD",
                            format!("{:.1} kN", results.total_load_kn),
                        );
                        chip(
                            ui,
                            "MAX |M|",
                            format!("{:.2} kNm", results.max_m),
                        );
                        chip(ui, "MAX UTILIZATION", format!("{:.2}", results.max_util));
                    }
                });
                ui.add_space(8.0);
                card_titled(ui, "Beam and loadings", |ui| {
                    draw_beam(ui, &self.project, self.results.as_ref());
                });
                ui.add_space(8.0);
                if let Some(results) = &self.results {
                    ui.columns(2, |cols| {
                        card_titled(&mut cols[0], "Bending moment", |ui| {
                            draw_diagram(
                                ui,
                                &self.project,
                                results,
                                ChartKind::Moment,
                                178.0,
                            );
                        });
                        card_titled(&mut cols[1], "Shear force", |ui| {
                            draw_diagram(
                                ui,
                                &self.project,
                                results,
                                ChartKind::Shear,
                                178.0,
                            );
                        });
                    });
                    ui.add_space(8.0);
                    card_titled(ui, "Deflection", |ui| {
                        draw_diagram(
                            ui,
                            &self.project,
                            results,
                            ChartKind::Deflection,
                            158.0,
                        );
                    });
                } else {
                    ui.label(
                        RichText::new("The model cannot be solved. Check the input list.")
                            .color(DANGER),
                    );
                }
                ui.add_space(8.0);
                card_titled(ui, "Model notes", |ui| {
                    ui.label(
                        RichText::new(
                            "Plane Euler-Bernoulli beam, linear elastic material. Vertical translation and rotation are the active degrees of freedom. A moment hinge splits the joint rotation into a left and a right value. Sliding bearings restrain vertical translation, fixed bearings add rotational restraint, and elastic bearings act as vertical springs.",
                        )
                        .size(10.5)
                        .color(MUTED),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Evidence alignment: overload/Überlastung is reported when utilization > 1.0 (D_TXT_UEBERLASTUNG concept); gamma_F default 1.5 matches D_TXT_GAMMA_1_5_FACH. Original CDbeam.CalcAuslastung subcase formulas, getter defaults, and some Eurocode/profile restrictions were not recovered (see app/MECHANICS.md).",
                        )
                        .size(9.8)
                        .color(MUTED),
                    );
                });
                ui.add_space(12.0);
            });
    }

    fn diagrams_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_source("diagram_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(results) = &self.results {
                    ui.horizontal_wrapped(|ui| {
                        chip(ui, "MAX |M|", format!("{:.2} kNm", results.max_m));
                        chip(ui, "MAX |V|", format!("{:.2} kN", results.max_v));
                        chip(ui, "MAX DEFLECTION", format!("{:.2} mm", results.max_defl));
                        chip(ui, "MAX ROTATION", format!("{:.5} rad", results.max_rot));
                    });
                    ui.add_space(8.0);
                    for kind in [
                        ChartKind::Moment,
                        ChartKind::Shear,
                        ChartKind::Deflection,
                        ChartKind::Rotation,
                    ] {
                        card_titled(ui, kind.title(), |ui| {
                            draw_diagram(ui, &self.project, results, kind, 190.0);
                        });
                        ui.add_space(8.0);
                    }
                    card_titled(ui, "Samples per member", |ui| {
                        egui::Grid::new("sample_table")
                            .num_columns(5)
                            .striped(true)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                for header in ["Member", "Start x", "End x", "M max", "V max"] {
                                    ui.label(RichText::new(header).size(9.5).strong());
                                }
                                ui.end_row();
                                for member in &results.members {
                                    ui.label(format!("{}", member.index + 1));
                                    ui.label(format!("{:.0}", member.x_start_mm));
                                    ui.label(format!("{:.0}", member.x_end_mm));
                                    ui.label(format!("{:.2}", member.m_max));
                                    ui.label(format!("{:.2}", member.v_max));
                                    ui.end_row();
                                }
                            });
                    });
                } else {
                    ui.label(RichText::new("No diagrams available.").color(DANGER));
                }
                ui.add_space(12.0);
            });
    }

    fn report_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Report studio")
                    .size(16.0)
                    .strong()
                    .color(INK),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let print = ui.add(
                    egui::Button::new(RichText::new("Print").color(Color32::WHITE))
                        .fill(ACCENT),
                );
                if print.clicked() {
                    self.print_report();
                }
                if ui.button("Export").clicked() {
                    self.export_report();
                }
            });
        });
        ui.label(
            RichText::new(
                "Choose format, page layout, sections and header text. The preview mirrors the printable HTML report.",
            )
            .size(10.0)
            .color(MUTED),
        );
        ui.add_space(6.0);
        let available = (ui.available_height() - 4.0).max(200.0);
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_source("report_options_scroll")
                .max_height(available)
                .auto_shrink([false, false])
                .show(&mut cols[0], |ui| {
                    self.report_options(ui);
                });
            egui::ScrollArea::vertical()
                .id_source("report_preview_scroll")
                .max_height(available)
                .auto_shrink([false, false])
                .show(&mut cols[1], |ui| {
                    if let Some(results) = &self.results {
                        report_preview(ui, &self.project, results);
                    } else {
                        ui.label(
                            RichText::new("Solve the beam to generate a report preview.")
                                .color(DANGER),
                        );
                    }
                });
        });
    }

    fn report_options(&mut self, ui: &mut egui::Ui) {
        let o = &mut self.project.report;
        egui::CollapsingHeader::new("Output")
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("report_output_grid")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Format");
                        egui::ComboBox::from_id_source("report_format")
                            .selected_text(o.format.label())
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for option in ReportFormat::all() {
                                    ui.selectable_value(&mut o.format, option.clone(), option.label());
                                }
                            });
                        ui.end_row();

                        ui.label("File name");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.file_stem).desired_width(170.0),
                        );
                        ui.end_row();

                        ui.label("Folder");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.output_dir)
                                .desired_width(170.0)
                                .hint_text("current folder"),
                        );
                        ui.end_row();

                        ui.label("Units");
                        egui::ComboBox::from_id_source("report_units")
                            .selected_text(o.units.label())
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for option in UnitSystem::all() {
                                    ui.selectable_value(&mut o.units, option.clone(), option.label());
                                }
                            });
                        ui.end_row();

                        ui.label("Decimals");
                        ui.add(egui::DragValue::new(&mut o.decimals).clamp_range(0..=6));
                        ui.end_row();

                        ui.label("Open after export");
                        ui.checkbox(&mut o.auto_open, "");
                        ui.end_row();

                        ui.label("Add timestamp to file");
                        ui.checkbox(&mut o.append_timestamp, "");
                        ui.end_row();
                    });
            });

        egui::CollapsingHeader::new("Page layout")
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("report_page_grid")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Paper");
                        egui::ComboBox::from_id_source("report_paper")
                            .selected_text(o.paper.label())
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for option in PaperSize::all() {
                                    ui.selectable_value(&mut o.paper, option.clone(), option.label());
                                }
                            });
                        ui.end_row();

                        ui.label("Landscape");
                        ui.checkbox(&mut o.landscape, "");
                        ui.end_row();

                        ui.label("Margin top");
                        ui.add(
                            egui::DragValue::new(&mut o.margin_top_mm)
                                .clamp_range(0.0..=60.0)
                                .speed(1.0)
                                .suffix(" mm"),
                        );
                        ui.end_row();

                        ui.label("Margin bottom");
                        ui.add(
                            egui::DragValue::new(&mut o.margin_bottom_mm)
                                .clamp_range(0.0..=60.0)
                                .speed(1.0)
                                .suffix(" mm"),
                        );
                        ui.end_row();

                        ui.label("Margin left");
                        ui.add(
                            egui::DragValue::new(&mut o.margin_left_mm)
                                .clamp_range(0.0..=60.0)
                                .speed(1.0)
                                .suffix(" mm"),
                        );
                        ui.end_row();

                        ui.label("Margin right");
                        ui.add(
                            egui::DragValue::new(&mut o.margin_right_mm)
                                .clamp_range(0.0..=60.0)
                                .speed(1.0)
                                .suffix(" mm"),
                        );
                        ui.end_row();

                        ui.label("Font");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.font_family).desired_width(170.0),
                        );
                        ui.end_row();

                        ui.label("Font size");
                        ui.add(
                            egui::DragValue::new(&mut o.font_size_pt)
                                .clamp_range(6.0..=18.0)
                                .speed(0.5)
                                .suffix(" pt"),
                        );
                        ui.end_row();

                        ui.label("Accent colour");
                        ui.color_edit_button_srgb(&mut o.accent);
                        ui.end_row();
                    });
            });

        egui::CollapsingHeader::new("Sections")
            .default_open(true)
            .show(ui, |ui| {
                ui.checkbox(&mut o.include_header, "Header and footer");
                ui.checkbox(&mut o.include_inputs, "Input data");
                ui.indent("input_children", |ui| {
                    ui.checkbox(&mut o.include_members, "Members");
                    ui.checkbox(&mut o.include_bearings, "Bearings");
                    ui.checkbox(&mut o.include_loads, "Loadings");
                });
                ui.checkbox(&mut o.include_results, "Results and reactions");
                ui.checkbox(&mut o.include_checks, "Design checks");
                ui.checkbox(&mut o.include_diagrams, "Diagrams");
                ui.indent("diagram_children", |ui| {
                    ui.checkbox(&mut o.include_moment, "Bending moment");
                    ui.checkbox(&mut o.include_shear, "Shear force");
                    ui.checkbox(&mut o.include_deflection, "Deflection");
                    ui.checkbox(&mut o.include_rotation, "Rotation");
                });
                ui.checkbox(&mut o.include_notes, "Notes");
                ui.checkbox(&mut o.include_signature, "Signature line");
                ui.checkbox(&mut o.page_numbers, "Page numbers");
            });

        egui::CollapsingHeader::new("Header and footer")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("report_text_grid")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Header left");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.header_left).desired_width(170.0),
                        );
                        ui.end_row();
                        ui.label("Header centre");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.header_center)
                                .desired_width(170.0),
                        );
                        ui.end_row();
                        ui.label("Header right");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.header_right).desired_width(170.0),
                        );
                        ui.end_row();
                        ui.label("Footer left");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.footer_left).desired_width(170.0),
                        );
                        ui.end_row();
                        ui.label("Footer centre");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.footer_center)
                                .desired_width(170.0),
                        );
                        ui.end_row();
                        ui.label("Footer right");
                        ui.add(
                            egui::TextEdit::singleline(&mut o.footer_right).desired_width(170.0),
                        );
                        ui.end_row();
                    });
                ui.label(
                    RichText::new(
                        "Tokens: &X title · &N project no. · &C customer · &E engineer · &D date · &T time · &F file · &P page · &R pages · &S page x of y",
                    )
                    .size(9.0)
                    .color(MUTED),
                );
            });

        egui::CollapsingHeader::new("Titles, notes and signature")
            .default_open(false)
            .show(ui, |ui| {
                text_field(ui, "Report title", &mut o.title);
                text_field(ui, "Subtitle", &mut o.subtitle);
                ui.label(RichText::new("Notes").size(10.5).color(MUTED));
                ui.add(
                    egui::TextEdit::multiline(&mut o.notes)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(4.0);
                text_field(ui, "Signature line", &mut o.signature_line);
            });

        ui.add_space(6.0);
        let path = report::output_path(&self.project);
        ui.label(
            RichText::new(format!("Output file: {}", path.display()))
                .size(9.5)
                .color(MUTED),
        );
        ui.label(
            RichText::new(format!(
                "Page: {} {} · margins {:.0}/{:.0}/{:.0}/{:.0} mm",
                self.project.report.paper.label(),
                if self.project.report.landscape {
                    "landscape"
                } else {
                    "portrait"
                },
                self.project.report.margin_top_mm,
                self.project.report.margin_right_mm,
                self.project.report.margin_bottom_mm,
                self.project.report.margin_left_mm
            ))
            .size(9.5)
            .color(MUTED),
        );
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        let mut open = self.about_open;
        egui::Window::new("About this rebuild")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(model::APP_NAME)
                        .size(16.0)
                        .strong()
                        .color(ACCENT),
                );
                ui.label(
                    RichText::new("A clean-room reconstruction of a continuous beam statics workflow.")
                        .size(10.5)
                        .color(MUTED),
                );
                ui.add_space(6.0);
                ui.label(
                    "Plane Euler-Bernoulli FEM with vertical and rotational degrees of freedom, sliding, fixed, hinged and elastic bearings, point, moment, trapezoid and concrete-pressure loads.",
                );
                ui.add_space(6.0);
                ui.label(
                    "Report output is generated as HTML, CSV or plain text. Printing opens the generated HTML in the system browser, where the print dialog and PDF export are available.",
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("No original artwork, logos or long text passages are used.")
                        .size(9.5)
                        .color(MUTED),
                );
            });
        self.about_open = open;
    }

    fn screenshot_tick(&mut self, ctx: &egui::Context) {
        if self.screenshot_path.is_none() {
            return;
        }
        self.screenshot_frames = self.screenshot_frames.saturating_add(1);
        ctx.request_repaint();
        if !self.screenshot_requested && self.screenshot_frames >= 120 {
            self.screenshot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
        if self.screenshot_requested {
            let image = ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(image) = image {
                if let Some(path) = &self.screenshot_path {
                    if let Some(parent) =
                        path.parent().filter(|p| !p.as_os_str().is_empty())
                    {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let saved = image::save_buffer(
                        path,
                        image.as_raw(),
                        image.size[0] as u32,
                        image.size[1] as u32,
                        image::ColorType::Rgba8,
                    );
                    match saved {
                        Ok(()) => {
                            eprintln!("screenshot saved: {}", path.display());
                            self.status = format!("Screenshot saved to {}", path.display());
                        }
                        Err(e) => {
                            eprintln!("screenshot failed: {e}");
                            self.status = format!("Screenshot failed: {e}");
                        }
                    }
                }
                self.screenshot_path = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                ctx.request_repaint();
            }
        }
    }
}

impl eframe::App for TipBeamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (results, error) = solve_now(&self.project);
        self.results = results;
        self.error = error;
        self.clamp_selections();

        self.top_bar(ctx);
        self.status_bar(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        self.central_panel(ctx);
        if self.about_open {
            self.about_window(ctx);
        }
        self.screenshot_tick(ctx);
    }
}

fn key_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.label(RichText::new(label).size(10.0).color(MUTED));
    ui.label(RichText::new(value).size(10.5).strong().color(INK));
    ui.end_row();
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        text.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

#[derive(Clone, Copy)]
enum ChartKind {
    Moment,
    Shear,
    Deflection,
    Rotation,
}

impl ChartKind {
    fn title(self) -> &'static str {
        match self {
            ChartKind::Moment => "Bending moment",
            ChartKind::Shear => "Shear force",
            ChartKind::Deflection => "Deflection",
            ChartKind::Rotation => "Rotation",
        }
    }

    fn unit(self) -> &'static str {
        match self {
            ChartKind::Moment => "kNm",
            ChartKind::Shear => "kN",
            ChartKind::Deflection => "mm",
            ChartKind::Rotation => "rad",
        }
    }

    fn color(self) -> Color32 {
        match self {
            ChartKind::Moment => Color32::from_rgb(45, 111, 159),
            ChartKind::Shear => Color32::from_rgb(164, 88, 42),
            ChartKind::Deflection => Color32::from_rgb(47, 125, 89),
            ChartKind::Rotation => Color32::from_rgb(106, 79, 159),
        }
    }

    fn value(self, sample: &Sample) -> f64 {
        match self {
            ChartKind::Moment => sample.m_knm,
            ChartKind::Shear => sample.v_kn,
            ChartKind::Deflection => sample.defl_mm,
            ChartKind::Rotation => sample.rot_rad,
        }
    }

    fn format(self, value: f64) -> String {
        match self {
            ChartKind::Moment => format!("{value:.2} kNm"),
            ChartKind::Shear => format!("{value:.2} kN"),
            ChartKind::Deflection => format!("{value:.2} mm"),
            ChartKind::Rotation => format!("{value:.5} rad"),
        }
    }
}

fn draw_beam(ui: &mut egui::Ui, project: &Project, results: Option<&Results>) {
    let width = ui.available_width().max(420.0);
    let height = 224.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, Color32::WHITE);
    painter.rect_stroke(rect, 8.0, Stroke::new(1.0, BORDER));

    let left = rect.left() + 52.0;
    let right = rect.right() - 24.0;
    let total = project.total_length_mm().max(1.0);
    let x = |mm: f64| -> f32 { left + ((mm / total) as f32) * (right - left) };
    let beam_y = rect.top() + 124.0;

    painter.line_segment(
        [pos2(left, beam_y + 3.0), pos2(right, beam_y + 3.0)],
        Stroke::new(2.0, Color32::from_rgb(228, 233, 238)),
    );
    painter.line_segment(
        [pos2(left, beam_y), pos2(right, beam_y)],
        Stroke::new(6.0, Color32::from_rgb(45, 58, 70)),
    );

    let mut start = 0.0;
    for (index, member) in project.members.iter().enumerate() {
        let a = x(start);
        let b = x(start + member.length_mm);
        let mid = (a + b) / 2.0;
        painter.line_segment(
            [pos2(a, beam_y - 14.0), pos2(a, beam_y + 18.0)],
            Stroke::new(1.0, Color32::from_rgb(140, 153, 166)),
        );
        if index + 1 == project.members.len() {
            painter.line_segment(
                [pos2(b, beam_y - 14.0), pos2(b, beam_y + 18.0)],
                Stroke::new(1.0, Color32::from_rgb(140, 153, 166)),
            );
        }
        if b - a > 54.0 {
            painter.text(
                pos2(mid, beam_y + 58.0),
                Align2::CENTER_CENTER,
                format!("{:.0} mm", member.length_mm),
                FontId::proportional(10.0),
                MUTED,
            );
            painter.text(
                pos2(mid, beam_y + 71.0),
                Align2::CENTER_CENTER,
                format!("Member {}", index + 1),
                FontId::proportional(9.0),
                Color32::from_rgb(146, 158, 170),
            );
        }
        start += member.length_mm;
    }

    for bearing in &project.bearings {
        let xx = x(project.node_x_mm(bearing.node));
        match bearing.kind {
            BearingKind::None => {}
            BearingKind::Sliding | BearingKind::Hinge => {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(xx, beam_y + 4.0),
                        pos2(xx - 12.0, beam_y + 32.0),
                        pos2(xx + 12.0, beam_y + 32.0),
                    ],
                    Color32::WHITE,
                    Stroke::new(1.6, Color32::from_rgb(45, 58, 70)),
                ));
                painter.circle_stroke(
                    pos2(xx - 6.0, beam_y + 35.0),
                    2.8,
                    Stroke::new(1.3, Color32::from_rgb(45, 58, 70)),
                );
                painter.circle_stroke(
                    pos2(xx + 6.0, beam_y + 35.0),
                    2.8,
                    Stroke::new(1.3, Color32::from_rgb(45, 58, 70)),
                );
            }
            BearingKind::Fixed => {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        pos2(xx - 7.0, beam_y + 3.0),
                        vec2(14.0, 30.0),
                    ),
                    2.0,
                    Color32::from_rgb(61, 75, 89),
                );
                for k in 0..4 {
                    painter.line_segment(
                        [
                            pos2(xx - 10.0 - k as f32 * 5.0, beam_y + 14.0 + k as f32 * 7.0),
                            pos2(xx - 2.0 - k as f32 * 5.0, beam_y + 20.0 + k as f32 * 7.0),
                        ],
                        Stroke::new(1.4, Color32::from_rgb(61, 75, 89)),
                    );
                }
            }
            BearingKind::Elastic => {
                let mut points = vec![pos2(xx, beam_y + 4.0)];
                for k in 0..5 {
                    let sign = if k % 2 == 0 { -1.0 } else { 1.0 };
                    points.push(pos2(xx + sign * 8.0, beam_y + 12.0 + k as f32 * 6.0));
                }
                points.push(pos2(xx, beam_y + 42.0));
                for pair in points.windows(2) {
                    painter.line_segment(
                        [pair[0], pair[1]],
                        Stroke::new(1.8, ACCENT),
                    );
                }
                painter.line_segment(
                    [pos2(xx - 12.0, beam_y + 42.0), pos2(xx + 12.0, beam_y + 42.0)],
                    Stroke::new(1.4, ACCENT),
                );
            }
        }
        if bearing.is_hinge() {
            painter.circle_filled(pos2(xx, beam_y), 4.6, Color32::WHITE);
            painter.circle_stroke(
                pos2(xx, beam_y),
                4.6,
                Stroke::new(1.4, Color32::from_rgb(45, 58, 70)),
            );
        }
    }

    for load in &project.loads {
        if !load.active {
            continue;
        }
        match load.kind {
            LoadKind::Point => {
                let xx = x(load.c);
                let top = rect.top() + 44.0;
                painter.line_segment(
                    [pos2(xx, top), pos2(xx, beam_y - 9.0)],
                    Stroke::new(1.8, LOAD_RED),
                );
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(xx, beam_y - 3.0),
                        pos2(xx - 4.5, beam_y - 11.0),
                        pos2(xx + 4.5, beam_y - 11.0),
                    ],
                    LOAD_RED,
                    Stroke::NONE,
                ));
                painter.text(
                    pos2(xx, top - 4.0),
                    Align2::CENTER_BOTTOM,
                    format!("P {:.1} kN", load.b),
                    FontId::proportional(9.5),
                    LOAD_RED,
                );
            }
            LoadKind::Moment => {
                let xx = x(load.c);
                let center = pos2(xx, beam_y - 42.0);
                painter.circle_stroke(center, 13.0, Stroke::new(1.6, LOAD_AMBER));
                painter.line_segment(
                    [pos2(xx - 4.0, beam_y - 30.0), pos2(xx + 4.0, beam_y - 30.0)],
                    Stroke::new(1.3, LOAD_AMBER),
                );
                painter.text(
                    pos2(xx, center.y - 18.0),
                    Align2::CENTER_BOTTOM,
                    format!("M {:.1} kNm", load.a),
                    FontId::proportional(9.5),
                    LOAD_AMBER,
                );
            }
            LoadKind::Trapezoid | LoadKind::ConcretePressure => {
                let x0 = x(load.c);
                let x1 = x((load.c + load.d).min(total));
                if x1 - x0 < 2.0 {
                    continue;
                }
                let (q0, q1) = if matches!(load.kind, LoadKind::ConcretePressure) {
                    (load.a * load.width_m(), load.b * load.width_m())
                } else {
                    (load.a, load.b)
                };
                let maxq = q0.abs().max(q1.abs()).max(0.5);
                let h0 = (q0.abs() / maxq) as f32 * 48.0;
                let h1 = (q1.abs() / maxq) as f32 * 48.0;
                let base = beam_y - 9.0;
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(x0, base),
                        pos2(x1, base),
                        pos2(x1, base - h1),
                        pos2(x0, base - h0),
                    ],
                    Color32::from_rgba_unmultiplied(232, 180, 74, 58),
                    Stroke::new(1.2, LOAD_AMBER),
                ));
                for k in 0..=10 {
                    let t = k as f32 / 10.0;
                    let xx = x0 + (x1 - x0) * t;
                    let h = h0 + (h1 - h0) * t;
                    painter.line_segment(
                        [pos2(xx, base), pos2(xx, base - h)],
                        Stroke::new(0.8, Color32::from_rgba_unmultiplied(184, 134, 43, 150)),
                    );
                }
                painter.text(
                    pos2((x0 + x1) / 2.0, rect.top() + 42.0),
                    Align2::CENTER_BOTTOM,
                    format!("q {:.1} / {:.1} kN/m", q0, q1),
                    FontId::proportional(9.5),
                    Color32::from_rgb(138, 98, 28),
                );
            }
        }
    }

    if let Some(results) = results {
        let status_color = if results.overloaded { DANGER } else { OK };
        painter.text(
            pos2(left, rect.bottom() - 14.0),
            Align2::LEFT_CENTER,
            format!(
                "max |M| {:.2} kNm    max |V| {:.2} kN    max deflection {:.2} mm",
                results.max_m, results.max_v, results.max_defl
            ),
            FontId::proportional(10.0),
            MUTED,
        );
        painter.text(
            pos2(right, rect.bottom() - 14.0),
            Align2::RIGHT_CENTER,
            if results.overloaded {
                format!("Overloaded · utilization {:.2}", results.max_util)
            } else {
                format!("Within capacity · utilization {:.2}", results.max_util)
            },
            FontId::proportional(10.0),
            status_color,
        );
    } else {
        painter.text(
            pos2(left, rect.bottom() - 14.0),
            Align2::LEFT_CENTER,
            "No solution",
            FontId::proportional(10.0),
            DANGER,
        );
    }
}

fn draw_diagram(
    ui: &mut egui::Ui,
    project: &Project,
    results: &Results,
    kind: ChartKind,
    height: f32,
) {
    let width = ui.available_width().max(280.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 7.0, Color32::WHITE);
    painter.rect_stroke(rect, 7.0, Stroke::new(1.0, BORDER));

    let left = rect.left() + 46.0;
    let right = rect.right() - 14.0;
    let top = rect.top() + 30.0;
    let bottom = rect.bottom() - 26.0;
    let total = project.total_length_mm().max(1.0);
    let x = |mm: f64| -> f32 { left + ((mm / total) as f32) * (right - left) };

    let mut points: Vec<(f64, f64)> = Vec::new();
    for member in &results.members {
        for sample in &member.samples {
            points.push((sample.x_mm, kind.value(sample)));
        }
    }
    if points.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No values",
            FontId::proportional(11.0),
            MUTED,
        );
        return;
    }

    let mut min = 0.0_f64;
    let mut max = 0.0_f64;
    for (_, value) in &points {
        min = min.min(*value);
        max = max.max(*value);
    }
    if (max - min).abs() < 1e-12 {
        max = 1.0;
        min = -1.0;
    }
    let pad = (max - min) * 0.10;
    let min_p = min - pad;
    let max_p = max + pad;
    let y = |value: f64| -> f32 {
        bottom - (((value - min_p) / (max_p - min_p)) as f32) * (bottom - top)
    };
    let zero = y(0.0);

    for step in 0..=3 {
        let yy = top + (bottom - top) * step as f32 / 3.0;
        painter.line_segment(
            [pos2(left, yy), pos2(right, yy)],
            Stroke::new(1.0, Color32::from_rgb(236, 240, 244)),
        );
    }
    painter.line_segment(
        [pos2(left, zero), pos2(right, zero)],
        Stroke::new(1.1, Color32::from_rgb(126, 140, 153)),
    );

    let mut joint = 0.0;
    for member in &project.members {
        let xx = x(joint);
        painter.line_segment(
            [pos2(xx, top), pos2(xx, bottom)],
            Stroke::new(1.0, Color32::from_rgb(232, 236, 240)),
        );
        joint += member.length_mm;
    }
    painter.line_segment(
        [pos2(right, top), pos2(right, bottom)],
        Stroke::new(1.0, Color32::from_rgb(232, 236, 240)),
    );

    let fill = Color32::from_rgba_unmultiplied(
        kind.color().r(),
        kind.color().g(),
        kind.color().b(),
        36,
    );
    for member in &results.members {
        for pair in member.samples.windows(2) {
            let x0 = x(pair[0].x_mm);
            let x1 = x(pair[1].x_mm);
            let v0 = kind.value(&pair[0]);
            let v1 = kind.value(&pair[1]);
            if v0 * v1 >= 0.0 {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(x0, zero),
                        pos2(x1, zero),
                        pos2(x1, y(v1)),
                        pos2(x0, y(v0)),
                    ],
                    fill,
                    Stroke::NONE,
                ));
            } else {
                let t = v0 / (v0 - v1);
                let xz = x0 + (x1 - x0) * t as f32;
                painter.add(egui::Shape::convex_polygon(
                    vec![pos2(x0, zero), pos2(xz, zero), pos2(x0, y(v0))],
                    fill,
                    Stroke::NONE,
                ));
                painter.add(egui::Shape::convex_polygon(
                    vec![pos2(xz, zero), pos2(x1, zero), pos2(x1, y(v1))],
                    fill,
                    Stroke::NONE,
                ));
            }
        }
    }

    for member in &results.members {
        let line: Vec<egui::Pos2> = member
            .samples
            .iter()
            .map(|sample| pos2(x(sample.x_mm), y(kind.value(sample))))
            .collect();
        if line.len() >= 2 {
            painter.add(egui::Shape::line(line, Stroke::new(1.8, kind.color())));
        }
    }

    painter.text(
        pos2(left, rect.top() + 13.0),
        Align2::LEFT_CENTER,
        kind.title(),
        FontId::proportional(11.5),
        INK,
    );
    painter.text(
        pos2(right, rect.top() + 13.0),
        Align2::RIGHT_CENTER,
        format!("[{}]", kind.unit()),
        FontId::proportional(9.5),
        MUTED,
    );
    painter.text(
        pos2(left, rect.bottom() - 12.0),
        Align2::LEFT_CENTER,
        format!("max {}", kind.format(max)),
        FontId::proportional(9.0),
        MUTED,
    );
    painter.text(
        pos2(right, rect.bottom() - 12.0),
        Align2::RIGHT_CENTER,
        format!("min {}", kind.format(min)),
        FontId::proportional(9.0),
        MUTED,
    );
    painter.text(
        pos2(left, bottom + 11.0),
        Align2::LEFT_CENTER,
        format!("0 {}", project.report.units.label().split(' ').next().unwrap_or("")),
        FontId::proportional(8.5),
        MUTED,
    );
    painter.text(
        pos2(right, bottom + 11.0),
        Align2::RIGHT_CENTER,
        format!("{:.0} mm", total),
        FontId::proportional(8.5),
        MUTED,
    );
}

fn report_preview(ui: &mut egui::Ui, project: &Project, results: &Results) {
    let options = &project.report;
    let page_width = ui.available_width().min(560.0);
    let blocks = report::preview_blocks(project, results);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "{} · {} · {} mm margins",
                options.paper.label(),
                if options.landscape {
                    "landscape"
                } else {
                    "portrait"
                },
                options.margin_left_mm
            ))
            .size(9.5)
            .color(MUTED),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new("Approximate page").size(9.5).color(MUTED));
        });
    });
    ui.add_space(4.0);
    egui::Frame::none()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0, Color32::from_rgb(203, 211, 220)))
        .rounding(3.0)
        .inner_margin(Margin::same(22.0))
        .show(ui, |ui| {
            let inner = (page_width - 46.0).max(220.0);
            ui.set_width(inner);
            ui.set_min_height(inner * 1.18);
            if options.include_header {
                let header = format!(
                    "{}    {}    {}",
                    report::token_text(project, &options.header_left, 1, 1),
                    report::token_text(project, &options.header_center, 1, 1),
                    report::token_text(project, &options.header_right, 1, 1)
                );
                ui.label(RichText::new(header).size(7.8).color(MUTED));
                ui.separator();
                ui.add_space(4.0);
            }
            for block in blocks {
                match block.kind {
                    report::PreviewKind::Title => {
                        ui.label(
                            RichText::new(block.text)
                                .size(20.0)
                                .strong()
                                .color(ACCENT),
                        );
                        ui.add_space(2.0);
                    }
                    report::PreviewKind::Subtitle => {
                        ui.label(RichText::new(block.text).size(10.5).color(MUTED));
                        ui.add_space(4.0);
                    }
                    report::PreviewKind::Heading => {
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(block.text)
                                .size(11.5)
                                .strong()
                                .color(ACCENT),
                        );
                        ui.separator();
                    }
                    report::PreviewKind::Body => {
                        ui.label(RichText::new(block.text).size(9.5).color(INK));
                    }
                    report::PreviewKind::TableHeader => {
                        ui.label(
                            RichText::new(block.text)
                                .monospace()
                                .size(8.2)
                                .strong()
                                .color(ACCENT),
                        );
                    }
                    report::PreviewKind::TableRow => {
                        ui.label(
                            RichText::new(block.text)
                                .monospace()
                                .size(8.2)
                                .color(INK),
                        );
                    }
                    report::PreviewKind::Note => {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(243, 246, 249))
                            .inner_margin(Margin::same(6.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(block.text).size(8.8).color(MUTED));
                            });
                    }
                    report::PreviewKind::Footer => {
                        ui.add_space(4.0);
                        ui.label(RichText::new(block.text).size(7.8).color(MUTED));
                    }
                }
            }
        });
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "The HTML report is the authoritative printable layout; this preview shows its structure and content.",
        )
        .size(9.0)
        .color(MUTED),
    );
}

fn main() -> eframe::Result<()> {
    let mut screenshot_path: Option<PathBuf> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--screenshot" {
            if let Some(path) = args.get(index + 1) {
                screenshot_path = Some(PathBuf::from(path));
                index += 2;
                continue;
            }
        } else if let Some(path) = arg.strip_prefix("--screenshot=") {
            screenshot_path = Some(PathBuf::from(path));
        }
        index += 1;
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1480.0, 940.0])
        .with_min_inner_size([1120.0, 720.0])
        .with_title(model::APP_NAME);
    if screenshot_path.is_some() {
        viewport = viewport
            .with_active(false)
            .with_window_level(egui::WindowLevel::AlwaysOnBottom);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let initial_shot = screenshot_path.clone();
    eframe::run_native(
        model::APP_NAME,
        native_options,
        Box::new(move |cc| Box::new(TipBeamApp::new(cc, initial_shot.clone()))),
    )
}

use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "Beam Statics Studio";
pub const APP_VERSION: &str = "0.1.0";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Timber,
    Steel,
    Aluminium,
    Other,
}

impl Material {
    pub fn label(&self) -> &'static str {
        match self {
            Material::Timber => "Timber",
            Material::Steel => "Steel",
            Material::Aluminium => "Aluminium",
            Material::Other => "Other",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Gauge {
    Default,
    TimberI20,
    Timber,
    Steel,
    Aluminium,
    Custom,
}

impl Gauge {
    pub fn label(&self) -> &'static str {
        match self {
            Gauge::Default => "Custom / default",
            Gauge::TimberI20 => "Timber I 20 (sample)",
            Gauge::Timber => "Timber section",
            Gauge::Steel => "Steel section",
            Gauge::Aluminium => "Aluminium section",
            Gauge::Custom => "Custom",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub length_mm: f64,
    pub i_cm4: f64,
    pub e_kn_cm2: f64,
    pub type_name: String,
    pub perm_m_knm: f64,
    pub perm_v_kn: f64,
    pub parts: u32,
    pub material: Material,
    pub self_weight_kn_m: f64,
    pub gauge: Gauge,
}

impl Default for Member {
    fn default() -> Self {
        Self {
            length_mm: 4000.0,
            i_cm4: 4500.0,
            e_kn_cm2: 965.0,
            type_name: "Timber I 20".to_string(),
            perm_m_knm: 5.00297,
            perm_v_kn: 10.9871,
            parts: 10,
            material: Material::Timber,
            self_weight_kn_m: 0.0,
            gauge: Gauge::TimberI20,
        }
    }
}

impl Member {
    pub fn apply_gauge(&mut self, gauge: Gauge) {
        self.gauge = gauge.clone();
        match gauge {
            Gauge::Default | Gauge::Custom => {}
            Gauge::TimberI20 => {
                self.i_cm4 = 4500.0;
                self.e_kn_cm2 = 965.0;
                self.type_name = "Timber I 20".to_string();
                self.perm_m_knm = 5.00297;
                self.perm_v_kn = 10.9871;
                self.parts = 10;
                self.material = Material::Timber;
            }
            Gauge::Timber => {
                self.i_cm4 = 2000.0;
                self.e_kn_cm2 = 1000.0;
                self.type_name = "Timber section".to_string();
                self.perm_m_knm = 4.0;
                self.perm_v_kn = 8.0;
                self.parts = 1;
                self.material = Material::Timber;
            }
            Gauge::Steel => {
                self.i_cm4 = 1200.0;
                self.e_kn_cm2 = 21000.0;
                self.type_name = "Steel section".to_string();
                self.perm_m_knm = 18.0;
                self.perm_v_kn = 40.0;
                self.parts = 1;
                self.material = Material::Steel;
            }
            Gauge::Aluminium => {
                self.i_cm4 = 1600.0;
                self.e_kn_cm2 = 7000.0;
                self.type_name = "Aluminium section".to_string();
                self.perm_m_knm = 9.0;
                self.perm_v_kn = 20.0;
                self.parts = 1;
                self.material = Material::Aluminium;
            }
        }
    }

    pub fn ei_knm2(&self) -> f64 {
        // I in cm4 -> m4 (1e-8); E in kN/cm2 -> kN/m2 (1e4)
        self.i_cm4 * 1e-8 * self.e_kn_cm2 * 1e4
    }

    pub fn length_m(&self) -> f64 {
        self.length_mm / 1000.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BearingKind {
    None,
    Sliding,
    Fixed,
    Hinge,
    Elastic,
}

impl BearingKind {
    pub fn label(&self) -> &'static str {
        match self {
            BearingKind::None => "No bearing",
            BearingKind::Sliding => "Sliding vertically",
            BearingKind::Fixed => "Fixed",
            BearingKind::Hinge => "Hinge",
            BearingKind::Elastic => "Elastic bearing",
        }
    }

    pub fn all() -> [BearingKind; 5] {
        [
            BearingKind::None,
            BearingKind::Sliding,
            BearingKind::Fixed,
            BearingKind::Hinge,
            BearingKind::Elastic,
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bearing {
    pub node: usize,
    pub kind: BearingKind,
    pub spring_kn_mm: f64,
    pub section_mm2: f64,
    pub e_kn_cm2: f64,
    pub length_mm: f64,
    pub horizontal: bool,
    pub moment_hinge: bool,
    pub comment: String,
}

impl Default for Bearing {
    fn default() -> Self {
        Self {
            node: 0,
            kind: BearingKind::Sliding,
            spring_kn_mm: 0.0,
            section_mm2: 0.0,
            e_kn_cm2: 0.0,
            length_mm: 0.0,
            horizontal: false,
            moment_hinge: false,
            comment: String::new(),
        }
    }
}

impl Bearing {
    pub fn vertical(&self) -> bool {
        !matches!(self.kind, BearingKind::None)
    }

    pub fn fixed_rotation(&self) -> bool {
        matches!(self.kind, BearingKind::Fixed)
    }

    pub fn is_hinge(&self) -> bool {
        self.moment_hinge || matches!(self.kind, BearingKind::Hinge)
    }

    pub fn spring_kn_m(&self) -> f64 {
        if matches!(self.kind, BearingKind::Elastic) {
            if self.spring_kn_mm > 0.0 {
                self.spring_kn_mm * 1000.0
            } else if self.section_mm2 > 0.0 && self.e_kn_cm2 > 0.0 && self.length_mm > 0.0 {
                // k = E*A/L, with E kN/cm2, A mm2, L mm -> kN/m
                10.0 * self.e_kn_cm2 * self.section_mm2 / self.length_mm
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    pub fn condition_label(&self) -> String {
        let mut parts = vec![self.kind.label().to_string()];
        if self.horizontal && !matches!(self.kind, BearingKind::Fixed) {
            parts.push("horizontal restraint".to_string());
        }
        if self.is_hinge() && !matches!(self.kind, BearingKind::Hinge) {
            parts.push("moment hinge".to_string());
        }
        if matches!(self.kind, BearingKind::Elastic) {
            parts.push(format!("k = {:.2} kN/mm", self.spring_kn_mm()));
        }
        parts.join(", ")
    }

    pub fn spring_kn_mm(&self) -> f64 {
        self.spring_kn_m() / 1000.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoadKind {
    Point,
    Moment,
    Trapezoid,
    ConcretePressure,
}

impl LoadKind {
    pub fn label(&self) -> &'static str {
        match self {
            LoadKind::Point => "Single load",
            LoadKind::Moment => "Single moment",
            LoadKind::Trapezoid => "Trapezoid load",
            LoadKind::ConcretePressure => "Concrete pressure",
        }
    }

    pub fn all() -> [LoadKind; 4] {
        [
            LoadKind::Point,
            LoadKind::Moment,
            LoadKind::Trapezoid,
            LoadKind::ConcretePressure,
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Load {
    pub kind: LoadKind,
    pub active: bool,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub comment: String,
}

impl Default for Load {
    fn default() -> Self {
        Self {
            kind: LoadKind::Point,
            active: true,
            a: 0.0,
            b: 10.0,
            c: 2000.0,
            d: 0.0,
            e: 0.0,
            comment: String::new(),
        }
    }
}

impl Load {
    pub fn summary(&self) -> String {
        match self.kind {
            LoadKind::Point => format!("P = {:.2} kN at {:.0} mm", self.b, self.c),
            LoadKind::Moment => format!("M = {:.2} kNm at {:.0} mm", self.a, self.c),
            LoadKind::Trapezoid => format!(
                "q = {:.2} / {:.2} kN/m, x = {:.0} mm, L = {:.0} mm",
                self.a, self.b, self.c, self.d
            ),
            LoadKind::ConcretePressure => format!(
                "p = {:.2} / {:.2} kN/m2, x = {:.0} mm, L = {:.0} mm",
                self.a, self.b, self.c, self.d
            ),
        }
    }

    pub fn width_m(&self) -> f64 {
        if self.e > 0.0 {
            self.e / 1000.0
        } else {
            1.0
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalcSettings {
    pub eurocode: bool,
    pub gamma_f: f64,
    pub gamma_m_steel: f64,
    pub gamma_m_timber: f64,
    pub kmod_timber: f64,
    pub interaction: Interaction,
    pub include_self_weight: bool,
    pub actual_load_gamma: bool,
}

impl Default for CalcSettings {
    fn default() -> Self {
        Self {
            eurocode: true,
            // Default load factor aligned with evidenced text D_TXT_GAMMA_1_5_FACH @ 0x140103ccc
            // ("gamma 1.5-fold"). Getter get_GammaF exists in original but its default value was
            // not recovered; other gamma/k_mod defaults here are engineering placeholders.
            gamma_f: 1.5,
            gamma_m_steel: 1.1,
            gamma_m_timber: 1.3,
            kmod_timber: 0.9,
            interaction: Interaction::None,
            include_self_weight: false,
            actual_load_gamma: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Interaction {
    None,
    Linear,
    Sqrt,
}

impl Interaction {
    pub fn label(&self) -> &'static str {
        match self {
            Interaction::None => "No M-V interaction",
            Interaction::Linear => "Linear M/Mper + V/Vper",
            Interaction::Sqrt => "Quadratic interaction",
        }
    }

    pub fn all() -> [Interaction; 3] {
        [Interaction::None, Interaction::Linear, Interaction::Sqrt]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UnitSystem {
    Metric,
    Si,
    Us,
}

impl UnitSystem {
    pub fn label(&self) -> &'static str {
        match self {
            UnitSystem::Metric => "Metric (mm, cm4, kN/cm2)",
            UnitSystem::Si => "SI (m, m4, GPa)",
            UnitSystem::Us => "US (ft, in4, ksi)",
        }
    }

    pub fn all() -> [UnitSystem; 3] {
        [UnitSystem::Metric, UnitSystem::Si, UnitSystem::Us]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    Html,
    Csv,
    Txt,
}

impl ReportFormat {
    pub fn label(&self) -> &'static str {
        match self {
            ReportFormat::Html => "HTML (print / PDF)",
            ReportFormat::Csv => "CSV",
            ReportFormat::Txt => "Plain text",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ReportFormat::Html => "html",
            ReportFormat::Csv => "csv",
            ReportFormat::Txt => "txt",
        }
    }

    pub fn all() -> [ReportFormat; 3] {
        [ReportFormat::Html, ReportFormat::Csv, ReportFormat::Txt]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportOptions {
    pub file_stem: String,
    pub output_dir: String,
    pub format: ReportFormat,
    pub units: UnitSystem,
    pub decimals: usize,
    pub paper: PaperSize,
    pub landscape: bool,
    pub margin_top_mm: f64,
    pub margin_left_mm: f64,
    pub margin_right_mm: f64,
    pub margin_bottom_mm: f64,
    pub font_family: String,
    pub font_size_pt: f64,
    pub accent: [u8; 3],
    pub title: String,
    pub subtitle: String,
    pub header_left: String,
    pub header_center: String,
    pub header_right: String,
    pub footer_left: String,
    pub footer_center: String,
    pub footer_right: String,
    pub include_header: bool,
    pub include_inputs: bool,
    pub include_members: bool,
    pub include_bearings: bool,
    pub include_loads: bool,
    pub include_results: bool,
    pub include_checks: bool,
    pub include_diagrams: bool,
    pub include_moment: bool,
    pub include_shear: bool,
    pub include_deflection: bool,
    pub include_rotation: bool,
    pub include_notes: bool,
    pub include_signature: bool,
    pub notes: String,
    pub signature_line: String,
    pub page_numbers: bool,
    pub auto_open: bool,
    pub append_timestamp: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            file_stem: "beam-statics-report".to_string(),
            output_dir: "output".to_string(),
            format: ReportFormat::Html,
            units: UnitSystem::Metric,
            decimals: 2,
            paper: PaperSize::A4,
            landscape: false,
            margin_top_mm: 25.0,
            margin_left_mm: 20.0,
            margin_right_mm: 20.0,
            margin_bottom_mm: 20.0,
            font_family: "Segoe UI".to_string(),
            font_size_pt: 10.0,
            accent: [32, 96, 140],
            title: "Beam Statics Report".to_string(),
            subtitle: "Continuous beam calculation".to_string(),
            header_left: "&X".to_string(),
            header_center: "&N".to_string(),
            header_right: "&D".to_string(),
            footer_left: "&C".to_string(),
            footer_center: "&S".to_string(),
            footer_right: "Page &P of &R".to_string(),
            include_header: true,
            include_inputs: true,
            include_members: true,
            include_bearings: true,
            include_loads: true,
            include_results: true,
            include_checks: true,
            include_diagrams: true,
            include_moment: true,
            include_shear: true,
            include_deflection: true,
            include_rotation: false,
            include_notes: true,
            include_signature: true,
            notes: "The calculation uses the Euler-Bernoulli beam model. Loads are entered in the original TipBeam units; report values follow the selected unit system.".to_string(),
            signature_line: "Prepared by: ______________________    Checked by: ______________________".to_string(),
            page_numbers: true,
            auto_open: true,
            append_timestamp: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaperSize {
    A4,
    A3,
    Letter,
    Legal,
}

impl PaperSize {
    pub fn label(&self) -> &'static str {
        match self {
            PaperSize::A4 => "A4",
            PaperSize::A3 => "A3",
            PaperSize::Letter => "Letter",
            PaperSize::Legal => "Legal",
        }
    }

    pub fn mm(&self) -> (f64, f64) {
        match self {
            PaperSize::A4 => (210.0, 297.0),
            PaperSize::A3 => (297.0, 420.0),
            PaperSize::Letter => (215.9, 279.4),
            PaperSize::Legal => (215.9, 355.6),
        }
    }

    pub fn all() -> [PaperSize; 4] {
        [
            PaperSize::A4,
            PaperSize::A3,
            PaperSize::Letter,
            PaperSize::Legal,
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub title: String,
    pub project_no: String,
    pub customer: String,
    pub engineer: String,
    pub comment: String,
    pub members: Vec<Member>,
    pub bearings: Vec<Bearing>,
    pub loads: Vec<Load>,
    pub calc: CalcSettings,
    pub report: ReportOptions,
}

impl Default for Project {
    fn default() -> Self {
        demo_project()
    }
}

pub fn demo_project() -> Project {
    let mut members = Vec::new();
    for _ in 0..4 {
        let mut m = Member::default();
        m.length_mm = 4000.0;
        m.type_name = "Timber I 20".to_string();
        members.push(m);
    }
    let bearings = vec![
        Bearing {
            node: 0,
            kind: BearingKind::Sliding,
            ..Default::default()
        },
        Bearing {
            node: 1,
            kind: BearingKind::Sliding,
            ..Default::default()
        },
        Bearing {
            node: 2,
            kind: BearingKind::Sliding,
            ..Default::default()
        },
        Bearing {
            node: 3,
            kind: BearingKind::Sliding,
            ..Default::default()
        },
        Bearing {
            node: 4,
            kind: BearingKind::Sliding,
            ..Default::default()
        },
    ];
    let loads = vec![
        Load {
            kind: LoadKind::Trapezoid,
            active: true,
            a: 5.0,
            b: 5.0,
            c: 0.0,
            d: 16000.0,
            ..Default::default()
        },
        Load {
            kind: LoadKind::Point,
            active: false,
            b: 12.0,
            c: 8000.0,
            ..Default::default()
        },
    ];
    Project {
        title: "Continuous beam - four equal spans".to_string(),
        project_no: "TB-001".to_string(),
        customer: "Example project".to_string(),
        engineer: "Engineering office".to_string(),
        comment: "Demo data: four equal spans, a timber I-section and a uniform load over the full length.".to_string(),
        members,
        bearings,
        loads,
        calc: CalcSettings::default(),
        report: ReportOptions::default(),
    }
}

impl Project {
    pub fn total_length_mm(&self) -> f64 {
        self.members.iter().map(|m| m.length_mm).sum()
    }

    pub fn total_length_m(&self) -> f64 {
        self.total_length_mm() / 1000.0
    }

    pub fn node_count(&self) -> usize {
        self.members.len() + 1
    }

    pub fn node_x_mm(&self, node: usize) -> f64 {
        self.members
            .iter()
            .take(node.min(self.members.len()))
            .map(|m| m.length_mm)
            .sum()
    }

    pub fn member_start_mm(&self, index: usize) -> f64 {
        self.members
            .iter()
            .take(index.min(self.members.len()))
            .map(|m| m.length_mm)
            .sum()
    }

    pub fn bearing_at(&self, node: usize) -> Option<&Bearing> {
        self.bearings.iter().find(|b| b.node == node)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.members.is_empty() {
            issues.push("Beam has no members.".to_string());
        }
        if self.members.iter().any(|m| m.length_mm <= 0.0) {
            issues.push("Member length not positive!".to_string());
        }
        if self.members.iter().any(|m| m.i_cm4 <= 0.0 || m.e_kn_cm2 <= 0.0) {
            issues.push("Moment of inertia and E module must be positive.".to_string());
        }
        if self.bearings.iter().any(|b| b.node >= self.node_count()) {
            issues.push("Bearing joint lies outside the beam.".to_string());
        }
        let total = self.total_length_mm();
        for l in &self.loads {
            if l.active
                && matches!(
                    l.kind,
                    LoadKind::Point | LoadKind::Moment | LoadKind::Trapezoid | LoadKind::ConcretePressure
                )
                && (l.c < 0.0 || l.c > total)
            {
                issues.push("Loading does not lie on beam!".to_string());
                break;
            }
        }
        issues
    }
}

use crate::model::{
    BearingKind, LoadKind, Material, Project, ReportFormat, ReportOptions, UnitSystem,
};
use crate::solver::{MemberResult, Results};

pub struct NumberFormat {
    pub units: UnitSystem,
    pub decimals: usize,
}

impl NumberFormat {
    pub fn new(units: &UnitSystem, decimals: usize) -> Self {
        Self {
            units: units.clone(),
            decimals: decimals.clamp(0, 6),
        }
    }

    fn n(&self, value: f64) -> String {
        format!("{:.*}", self.decimals, value)
    }

    pub fn length(&self, mm: f64) -> String {
        match self.units {
            UnitSystem::Metric => self.n(mm),
            UnitSystem::Si => self.n(mm / 1000.0),
            UnitSystem::Us => self.n(mm / 304.8),
        }
    }

    pub fn length_unit(&self) -> &'static str {
        match self.units {
            UnitSystem::Metric => "mm",
            UnitSystem::Si => "m",
            UnitSystem::Us => "ft",
        }
    }

    pub fn deflection(&self, mm: f64) -> String {
        match self.units {
            UnitSystem::Metric => self.n(mm),
            UnitSystem::Si => self.n(mm),
            UnitSystem::Us => self.n(mm / 25.4),
        }
    }

    pub fn deflection_unit(&self) -> &'static str {
        match self.units {
            UnitSystem::Metric => "mm",
            UnitSystem::Si => "mm",
            UnitSystem::Us => "in",
        }
    }

    pub fn inertia(&self, cm4: f64) -> String {
        match self.units {
            UnitSystem::Metric => self.n(cm4),
            UnitSystem::Si => self.n(cm4 * 1e-8),
            UnitSystem::Us => self.n(cm4 / 41.623_143_041_6),
        }
    }

    pub fn inertia_unit(&self) -> &'static str {
        match self.units {
            UnitSystem::Metric => "cm4",
            UnitSystem::Si => "m4",
            UnitSystem::Us => "in4",
        }
    }

    pub fn e_module(&self, kn_cm2: f64) -> String {
        match self.units {
            UnitSystem::Metric => self.n(kn_cm2),
            UnitSystem::Si => self.n(kn_cm2 / 10.0),
            UnitSystem::Us => self.n(kn_cm2 * 1.450_377_438),
        }
    }

    pub fn e_unit(&self) -> &'static str {
        match self.units {
            UnitSystem::Metric => "kN/cm2",
            UnitSystem::Si => "GPa",
            UnitSystem::Us => "ksi",
        }
    }

    pub fn force(&self, kn: f64) -> String {
        match self.units {
            UnitSystem::Metric | UnitSystem::Si => self.n(kn),
            UnitSystem::Us => self.n(kn * 0.224_808_943),
        }
    }

    pub fn force_unit(&self) -> &'static str {
        match self.units {
            UnitSystem::Metric | UnitSystem::Si => "kN",
            UnitSystem::Us => "kip",
        }
    }

    pub fn moment(&self, knm: f64) -> String {
        match self.units {
            UnitSystem::Metric | UnitSystem::Si => self.n(knm),
            UnitSystem::Us => self.n(knm * 0.737_562_149),
        }
    }

    pub fn moment_unit(&self) -> &'static str {
        match self.units {
            UnitSystem::Metric | UnitSystem::Si => "kNm",
            UnitSystem::Us => "kip-ft",
        }
    }

    pub fn line_load(&self, kn_m: f64) -> String {
        match self.units {
            UnitSystem::Metric | UnitSystem::Si => self.n(kn_m),
            UnitSystem::Us => self.n(kn_m * 0.068_521_765),
        }
    }

    pub fn line_load_unit(&self) -> &'static str {
        match self.units {
            UnitSystem::Metric | UnitSystem::Si => "kN/m",
            UnitSystem::Us => "kip/ft",
        }
    }

    pub fn pressure(&self, kn_m2: f64) -> String {
        match self.units {
            UnitSystem::Metric | UnitSystem::Si => self.n(kn_m2),
            UnitSystem::Us => self.n(kn_m2 * 0.145_037_738),
        }
    }

    pub fn pressure_unit(&self) -> &'static str {
        match self.units {
            UnitSystem::Metric | UnitSystem::Si => "kN/m2",
            UnitSystem::Us => "psi",
        }
    }

    pub fn rotation(&self, rad: f64) -> String {
        self.n(rad)
    }
}

pub fn output_path(project: &Project) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::new();
    if !project.report.output_dir.trim().is_empty() {
        path.push(project.report.output_dir.trim());
    }
    let stem_raw = if project.report.file_stem.trim().is_empty() {
        "beam-statics-report"
    } else {
        project.report.file_stem.trim()
    };
    let stem = if project.report.append_timestamp {
        let now = chrono::Local::now();
        format!("{}-{}", stem_raw, now.format("%Y%m%d-%H%M%S"))
    } else {
        stem_raw.to_string()
    };
    path.push(format!("{}.{}", stem, project.report.format.extension()));
    path
}

pub fn write_report(project: &Project, results: &Results) -> Result<std::path::PathBuf, String> {
    let path = output_path(project);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let text = match project.report.format {
        ReportFormat::Html => html(project, results),
        ReportFormat::Csv => csv(project, results),
        ReportFormat::Txt => txt(project, results),
    };
    std::fs::write(&path, text).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    if project.report.auto_open {
        let _ = open_external(&path);
    }
    Ok(path)
}

pub fn open_external(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Could not open the report: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        std::process::Command::new(opener)
            .arg(path)
            .spawn()
            .map_err(|e| format!("Could not open the report: {e}"))?;
        Ok(())
    }
}

pub fn print_report(project: &Project, results: &Results) -> Result<std::path::PathBuf, String> {
    let mut print_project = project.clone();
    print_project.report.format = ReportFormat::Html;
    let path = output_path(&print_project);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, html(&print_project, results))
        .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    if print_project.report.auto_open {
        open_external(&path)?;
    }
    Ok(path)
}

pub fn token_text(project: &Project, text: &str, page: usize, pages: usize) -> String {
    let now = chrono::Local::now();
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('&') => out.push('&'),
            Some('X') => out.push_str(&project.report.title),
            Some('N') => out.push_str(&project.project_no),
            Some('C') => out.push_str(&project.customer),
            Some('E') => out.push_str(&project.engineer),
            Some('D') => out.push_str(&now.format("%Y-%m-%d").to_string()),
            Some('T') => out.push_str(&now.format("%H:%M").to_string()),
            Some('F') => out.push_str(&project.report.file_stem),
            Some('P') => out.push_str(&page.to_string()),
            Some('R') => out.push_str(&pages.to_string()),
            Some('S') => out.push_str(&format!("Page {page} of {pages}")),
            Some(other) => {
                out.push('&');
                out.push(other);
            }
            None => out.push('&'),
        }
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn html(project: &Project, results: &Results) -> String {
    let o = &project.report;
    let f = NumberFormat::new(&o.units, o.decimals);
    let (w, h) = o.paper.mm();
    let (pw, ph) = if o.landscape { (h, w) } else { (w, h) };
    let accent = format!("#{:02x}{:02x}{:02x}", o.accent[0], o.accent[1], o.accent[2]);
    let header_left = html_escape(&token_text(project, &o.header_left, 1, 1));
    let header_center = html_escape(&token_text(project, &o.header_center, 1, 1));
    let header_right = html_escape(&token_text(project, &o.header_right, 1, 1));
    let footer_left = html_escape(&token_text(project, &o.footer_left, 1, 1));
    let footer_center = html_escape(&token_text(project, &o.footer_center, 1, 1));
    let footer_right = html_escape(&token_text(project, &o.footer_right, 1, 1));

    let mut s = String::new();
    s.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    s.push_str(&format!("<title>{}</title>\n", html_escape(&o.title)));
    s.push_str("<style>\n");
    s.push_str(&format!(
        "@page {{ size: {:.2}mm {:.2}mm; margin: {:.1}mm {:.1}mm {:.1}mm {:.1}mm; }}\n",
        pw, ph, o.margin_top_mm, o.margin_right_mm, o.margin_bottom_mm, o.margin_left_mm
    ));
    s.push_str(&format!(
        "body {{ font-family: '{}', 'Segoe UI', sans-serif; font-size: {:.1}pt; color:#20242a; background:#eceff3; margin:0; }}\n",
        html_escape(&o.font_family), o.font_size_pt
    ));
    s.push_str(".sheet { width: 210mm; min-height: 297mm; box-sizing: border-box; padding: 25mm 20mm 20mm 20mm; margin: 18px auto; background:#fff; box-shadow:0 8px 30px rgba(20,30,45,.18); }\n");
    s.push_str(".topbar { max-width: 210mm; margin: 14px auto -6px auto; display:flex; justify-content:flex-end; gap:8px; }\n");
    s.push_str(".topbar button { background:#fff; border:1px solid #b9c3cf; border-radius:6px; padding:7px 12px; cursor:pointer; }\n");
    s.push_str(&format!("h1 {{ color:{}; font-size: 23pt; margin:0 0 2mm 0; letter-spacing:-.3px; }}\n", accent));
    s.push_str(".subtitle { color:#5a6675; font-size:11pt; margin-bottom:6mm; }\n");
    s.push_str(".meta { display:grid; grid-template-columns: 1fr 1fr; gap:2mm 8mm; margin:5mm 0 7mm 0; }\n");
    s.push_str(".meta .k { color:#6a7480; font-size:8.5pt; text-transform:uppercase; letter-spacing:.6px; }\n");
    s.push_str(".meta .v { font-weight:600; }\n");
    s.push_str(&format!("h2 {{ color:{}; font-size:12.5pt; margin:7mm 0 2mm 0; border-bottom:1.5px solid {}; padding-bottom:1.5mm; }}\n", accent, accent));
    s.push_str("h3 { font-size:10.5pt; margin:4mm 0 1.5mm 0; color:#39434f; }\n");
    s.push_str("table { width:100%; border-collapse:collapse; margin:1mm 0 4mm 0; font-size:8.7pt; }\n");
    s.push_str(&format!("th {{ background:{}; color:#fff; text-align:left; padding:2mm 1.6mm; font-weight:600; }}\n", accent));
    s.push_str("td { padding:1.5mm 1.6mm; border-bottom:1px solid #dce2e9; vertical-align:top; }\n");
    s.push_str("tr:nth-child(even) td { background:#f6f8fa; }\n");
    s.push_str(".num { text-align:right; font-variant-numeric: tabular-nums; }\n");
    s.push_str(".ok { color:#176b3a; font-weight:700; }\n.err { color:#a42323; font-weight:700; }\n");
    s.push_str(".note { background:#f4f7fa; border-left:3px solid #9fb2c4; padding:3mm 4mm; margin:4mm 0; font-size:9pt; }\n");
    s.push_str(".diagram { margin:3mm 0 6mm 0; page-break-inside:avoid; }\n.diagram svg { width:100%; height:auto; border:1px solid #dce2e9; background:#fff; }\n");
    s.push_str(".foot { display:flex; justify-content:space-between; color:#6a7480; font-size:8pt; border-top:1px solid #dce2e9; margin-top:8mm; padding-top:2mm; }\n");
    s.push_str(".head { display:flex; justify-content:space-between; color:#6a7480; font-size:8pt; border-bottom:1px solid #dce2e9; padding-bottom:2mm; margin-bottom:6mm; }\n");
    s.push_str(".sig { margin-top:12mm; color:#4a5663; font-size:9pt; }\n");
    s.push_str("@media print { body { background:#fff; } .sheet { width:auto; min-height:0; margin:0; padding:0; box-shadow:none; } .topbar { display:none; } tr { page-break-inside:avoid; } }\n");
    s.push_str("</style>\n</head>\n<body>\n");
    s.push_str("<div class=\"topbar no-print\"><button onclick=\"window.print()\">Print / Save as PDF</button></div>\n");
    s.push_str("<div class=\"sheet\">\n");

    if o.include_header {
        s.push_str(&format!(
            "<div class=\"head\"><span>{header_left}</span><span>{header_center}</span><span>{header_right}</span></div>\n"
        ));
    }
    s.push_str(&format!("<h1>{}</h1>\n", html_escape(&o.title)));
    if !o.subtitle.is_empty() {
        s.push_str(&format!("<div class=\"subtitle\">{}</div>\n", html_escape(&o.subtitle)));
    }
    s.push_str("<div class=\"meta\">\n");
    meta_pair(&mut s, "Project", &project.title);
    meta_pair(&mut s, "Project number", &project.project_no);
    meta_pair(&mut s, "Customer", &project.customer);
    meta_pair(&mut s, "Engineer in charge", &project.engineer);
    meta_pair(&mut s, "Date", &chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
    meta_pair(&mut s, "File", &format!("{}.{}", o.file_stem, o.format.extension()));
    s.push_str("</div>\n");

    if !project.comment.trim().is_empty() {
        s.push_str(&format!("<div class=\"note\">{}</div>\n", html_escape(&project.comment)));
    }

    s.push_str("<h2>1. Calculation basis</h2>\n");
    s.push_str("<table><tbody>\n");
    row2(&mut s, "Calculation model", "Euler-Bernoulli continuous beam, linear elastic");
    row2(
        &mut s,
        "Design check",
        if project.calc.eurocode {
            "Eurocode partial factors (evidenced: D_TXT_RECHNUNG_NACH_EUROCODE)"
        } else {
            "Permissible working loads"
        },
    );
    row2(
        &mut s,
        "Partial factor for loads gamma_F",
        &format!(
            "{:.3} (evidenced text: D_TXT_GAMMA_1_5_FACH @ 0x140103ccc)",
            project.calc.gamma_f
        ),
    );
    row2(&mut s, "Material factor steel gamma_M", &format!("{:.3}", project.calc.gamma_m_steel));
    row2(&mut s, "Material factor timber gamma_M", &format!("{:.3}", project.calc.gamma_m_timber));
    row2(&mut s, "Modification factor timber k_mod", &format!("{:.3}", project.calc.kmod_timber));
    row2(&mut s, "M-V interaction", project.calc.interaction.label());
    row2(&mut s, "Units", o.units.label());
    s.push_str("</tbody></table>\n");

    if o.include_inputs {
        if o.include_members {
            s.push_str("<h2>2. Members</h2>\n");
            s.push_str(&format!(
                "<table><thead><tr><th>No.</th><th>Type</th><th class=\"num\">Length [{}]</th><th class=\"num\">I [{}]</th><th class=\"num\">E [{}]</th><th class=\"num\">M per [{}]</th><th class=\"num\">V per [{}]</th><th class=\"num\">Parts</th><th>Material</th></tr></thead><tbody>\n",
                f.length_unit(), f.inertia_unit(), f.e_unit(), f.moment_unit(), f.force_unit()
            ));
            for (i, m) in project.members.iter().enumerate() {
                s.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td>{}</td></tr>\n",
                    i + 1,
                    html_escape(&m.type_name),
                    f.length(m.length_mm),
                    f.inertia(m.i_cm4),
                    f.e_module(m.e_kn_cm2),
                    f.moment(m.perm_m_knm),
                    f.force(m.perm_v_kn),
                    m.parts,
                    m.material.label()
                ));
            }
            s.push_str("</tbody></table>\n");
        }
        if o.include_bearings {
            s.push_str("<h3>Bearings</h3>\n");
            s.push_str("<table><thead><tr><th>No.</th><th>Joint</th><th>Condition</th><th class=\"num\">Position [");
            s.push_str(f.length_unit());
            s.push_str("]</th><th class=\"num\">Spring [");
            s.push_str(f.force_unit());
            s.push_str("/");
            s.push_str(f.length_unit());
            s.push_str("]</th><th>Comment</th></tr></thead><tbody>\n");
            for (i, b) in project.bearings.iter().enumerate() {
                s.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td>{}</td></tr>\n",
                    i + 1,
                    b.node + 1,
                    html_escape(&b.condition_label()),
                    f.length(project.node_x_mm(b.node)),
                    if matches!(b.kind, BearingKind::Elastic) { f.force(b.spring_kn_mm()) } else { "-".to_string() },
                    html_escape(&b.comment)
                ));
            }
            s.push_str("</tbody></table>\n");
        }
        if o.include_loads {
            s.push_str("<h3>Loadings</h3>\n");
            s.push_str("<table><thead><tr><th>No.</th><th>Type</th><th>Active</th><th>Values</th><th>Comment</th></tr></thead><tbody>\n");
            for (i, l) in project.loads.iter().enumerate() {
                s.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                    i + 1,
                    l.kind.label(),
                    if l.active { "yes" } else { "no" },
                    html_escape(&load_text(l, &f)),
                    html_escape(&l.comment)
                ));
            }
            s.push_str("</tbody></table>\n");
        }
    }

    if o.include_results {
        s.push_str("<h2>3. Results</h2>\n");
        s.push_str("<table><tbody>\n");
        row2(&mut s, "Total beam length", &format!("{} {}", f.length(project.total_length_mm()), f.length_unit()));
        row2(&mut s, "Total vertical load", &format!("{} {}", f.force(results.total_load_kn), f.force_unit()));
        row2(&mut s, "Sum of reactions", &format!("{} {}", f.force(results.sum_reactions_kn), f.force_unit()));
        row2(&mut s, "Maximum bending moment |M|", &format!("{} {}", f.moment(results.max_m), f.moment_unit()));
        row2(&mut s, "Maximum shear force |V|", &format!("{} {}", f.force(results.max_v), f.force_unit()));
        row2(&mut s, "Maximum deflection", &format!("{} {}", f.deflection(results.max_defl), f.deflection_unit()));
        row2(&mut s, "Maximum rotation", &format!("{} rad", f.rotation(results.max_rot)));
        row2(&mut s, "Maximum utilization", &format!("{:.2}", results.max_util));
        row2(&mut s, "Status", if results.overloaded { "Overloaded !" } else { "Within capacity" });
        s.push_str("</tbody></table>\n");

        s.push_str("<h3>Member results</h3>\n");
        s.push_str(&format!(
            "<table><thead><tr><th>No.</th><th class=\"num\">M max [{}]</th><th class=\"num\">M min [{}]</th><th class=\"num\">V max [{}]</th><th class=\"num\">V min [{}]</th><th class=\"num\">Deflection [{}]</th><th class=\"num\">Util. M</th><th class=\"num\">Util. V</th><th>Status</th></tr></thead><tbody>\n",
            f.moment_unit(), f.moment_unit(), f.force_unit(), f.force_unit(), f.deflection_unit()
        ));
        for r in &results.members {
            s.push_str(&format!(
                "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{:.2}</td><td class=\"num\">{:.2}</td><td class=\"{}\">{}</td></tr>\n",
                r.index + 1,
                f.moment(r.m_max),
                f.moment(r.m_min),
                f.force(r.v_max),
                f.force(r.v_min),
                f.deflection(r.defl_max.abs().max(r.defl_min.abs())),
                r.util_m,
                r.util_v,
                if r.overloaded { "err" } else { "ok" },
                if r.overloaded { "Overloaded !" } else { "OK" }
            ));
        }
        s.push_str("</tbody></table>\n");

        s.push_str("<h3>Reactions</h3>\n");
        s.push_str(&format!(
            "<table><thead><tr><th>Joint</th><th class=\"num\">Position [{}]</th><th class=\"num\">Vertical reaction [{}]</th><th class=\"num\">Moment [{}]</th><th>Bearing</th></tr></thead><tbody>\n",
            f.length_unit(), f.force_unit(), f.moment_unit()
        ));
        for r in &results.reactions {
            s.push_str(&format!(
                "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td>{}</td></tr>\n",
                r.node + 1,
                f.length(r.x_mm),
                f.force(r.force_kn),
                f.moment(r.moment_knm),
                html_escape(&r.kind)
            ));
        }
        s.push_str("</tbody></table>\n");
    }

    if o.include_diagrams
        && (o.include_moment || o.include_shear || o.include_deflection || o.include_rotation)
    {
        s.push_str("<h2>4. Diagrams</h2>\n");
        s.push_str("<div class=\"diagram\">\n");
        s.push_str(&beam_svg(project, results, &f));
        s.push_str("</div>\n");
        if o.include_moment {
            s.push_str("<div class=\"diagram\">\n");
            s.push_str(&diagram_svg(project, results, DiagramKind::Moment, &f));
            s.push_str("</div>\n");
        }
        if o.include_shear {
            s.push_str("<div class=\"diagram\">\n");
            s.push_str(&diagram_svg(project, results, DiagramKind::Shear, &f));
            s.push_str("</div>\n");
        }
        if o.include_deflection {
            s.push_str("<div class=\"diagram\">\n");
            s.push_str(&diagram_svg(project, results, DiagramKind::Deflection, &f));
            s.push_str("</div>\n");
        }
        if o.include_rotation {
            s.push_str("<div class=\"diagram\">\n");
            s.push_str(&diagram_svg(project, results, DiagramKind::Rotation, &f));
            s.push_str("</div>\n");
        }
    }

    if o.include_checks {
        s.push_str("<h2>5. Checks</h2>\n");
        s.push_str("<table><tbody>\n");
        row2(&mut s, "Governing member", &format!("Member {}", results.max_util_member + 1));
        row2(&mut s, "Maximum utilization", &format!("{:.3}", results.max_util));
        row2(&mut s, "Utilization limit", "1.000");
        row2(
            &mut s,
            "Result",
            if results.overloaded {
                "Overloaded !"
            } else {
                "All checks satisfied"
            },
        );
        if project.calc.interaction == crate::model::Interaction::None {
            row2(&mut s, "M-V interaction", "no M-V interaction");
        }
        s.push_str("</tbody></table>\n");
    }

    if o.include_notes && !o.notes.trim().is_empty() {
        s.push_str("<h2>6. Notes</h2>\n");
        s.push_str(&format!("<div class=\"note\">{}</div>\n", html_escape(&o.notes)));
    }
    if o.include_signature {
        s.push_str(&format!("<div class=\"sig\">{}</div>\n", html_escape(&o.signature_line)));
    }

    if o.include_header {
        s.push_str(&format!(
            "<div class=\"foot\"><span>{footer_left}</span><span>{footer_center}</span><span>{footer_right}</span></div>\n"
        ));
    }
    s.push_str("</div>\n</body>\n</html>\n");
    s
}

fn meta_pair(s: &mut String, k: &str, v: &str) {
    s.push_str(&format!(
        "<div><div class=\"k\">{}</div><div class=\"v\">{}</div></div>\n",
        html_escape(k),
        html_escape(v)
    ));
}

fn row2(s: &mut String, k: &str, v: &str) {
    s.push_str(&format!(
        "<tr><td style=\"width:38%;color:#5a6675\">{}</td><td>{}</td></tr>\n",
        html_escape(k),
        html_escape(v)
    ));
}

fn load_text(l: &crate::model::Load, f: &NumberFormat) -> String {
    match l.kind {
        LoadKind::Point => format!(
            "P = {} {} at {} {}",
            f.force(l.b),
            f.force_unit(),
            f.length(l.c),
            f.length_unit()
        ),
        LoadKind::Moment => format!(
            "M = {} {} at {} {}",
            f.moment(l.a),
            f.moment_unit(),
            f.length(l.c),
            f.length_unit()
        ),
        LoadKind::Trapezoid => format!(
            "q = {} / {} {} from {} {} over {} {}",
            f.line_load(l.a),
            f.line_load(l.b),
            f.line_load_unit(),
            f.length(l.c),
            f.length_unit(),
            f.length(l.d),
            f.length_unit()
        ),
        LoadKind::ConcretePressure => format!(
            "p = {} / {} {} from {} {} over {} {}, width {} {}",
            f.pressure(l.a),
            f.pressure(l.b),
            f.pressure_unit(),
            f.length(l.c),
            f.length_unit(),
            f.length(l.d),
            f.length_unit(),
            f.length(l.e),
            f.length_unit()
        ),
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn csv(project: &Project, results: &Results) -> String {
    let f = NumberFormat::new(&project.report.units, project.report.decimals);
    let mut s = String::new();
    s.push_str("Section,Item,Field,Value,Unit\n");
    let meta = [
        ("Project", "Title", project.title.clone()),
        ("Project", "Number", project.project_no.clone()),
        ("Project", "Customer", project.customer.clone()),
        ("Project", "Engineer", project.engineer.clone()),
        ("Project", "Date", chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
        ("Calculation", "Total length", f.length(project.total_length_mm())),
        ("Calculation", "Total load", f.force(results.total_load_kn)),
        ("Calculation", "Sum reactions", f.force(results.sum_reactions_kn)),
        ("Calculation", "Max moment", f.moment(results.max_m)),
        ("Calculation", "Max shear", f.force(results.max_v)),
        ("Calculation", "Max deflection", f.deflection(results.max_defl)),
        ("Calculation", "Max rotation", f.rotation(results.max_rot)),
        ("Calculation", "Max utilization", format!("{:.3}", results.max_util)),
        ("Calculation", "Status", if results.overloaded { "Overloaded" } else { "OK" }.to_string()),
    ];
    for (sec, field, value) in meta {
        s.push_str(&format!(
            "{},{},{},{},\n",
            csv_escape(sec),
            csv_escape(field),
            csv_escape(&value),
            ""
        ));
    }
    s.push('\n');
    s.push_str("Members\nNo,Type,Length,I,E,Mper,Vper,Parts,Material\n");
    for (i, m) in project.members.iter().enumerate() {
        s.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            i + 1,
            csv_escape(&m.type_name),
            f.length(m.length_mm),
            f.inertia(m.i_cm4),
            f.e_module(m.e_kn_cm2),
            f.moment(m.perm_m_knm),
            f.force(m.perm_v_kn),
            m.parts,
            m.material.label()
        ));
    }
    s.push('\n');
    s.push_str("Bearings\nNo,Joint,Condition,Position,Spring,Comment\n");
    for (i, b) in project.bearings.iter().enumerate() {
        s.push_str(&format!(
            "{},{},{},{},{},{}\n",
            i + 1,
            b.node + 1,
            csv_escape(&b.condition_label()),
            f.length(project.node_x_mm(b.node)),
            f.force(b.spring_kn_mm()),
            csv_escape(&b.comment)
        ));
    }
    s.push('\n');
    s.push_str("Loadings\nNo,Type,Active,Values,Comment\n");
    for (i, l) in project.loads.iter().enumerate() {
        s.push_str(&format!(
            "{},{},{},{},{}\n",
            i + 1,
            l.kind.label(),
            if l.active { "yes" } else { "no" },
            csv_escape(&load_text(l, &f)),
            csv_escape(&l.comment)
        ));
    }
    s.push('\n');
    s.push_str("Member results\nNo,Mmax,Mmin,Vmax,Vmin,Deflection,UtilM,UtilV,Status\n");
    for r in &results.members {
        s.push_str(&format!(
            "{},{},{},{},{},{},{:.3},{:.3},{}\n",
            r.index + 1,
            f.moment(r.m_max),
            f.moment(r.m_min),
            f.force(r.v_max),
            f.force(r.v_min),
            f.deflection(r.defl_max.abs().max(r.defl_min.abs())),
            r.util_m,
            r.util_v,
            if r.overloaded { "Overloaded" } else { "OK" }
        ));
    }
    s.push('\n');
    s.push_str("Reactions\nJoint,Position,Vertical,Moment,Bearing\n");
    for r in &results.reactions {
        s.push_str(&format!(
            "{},{},{},{},{}\n",
            r.node + 1,
            f.length(r.x_mm),
            f.force(r.force_kn),
            f.moment(r.moment_knm),
            csv_escape(&r.kind)
        ));
    }
    s
}

fn txt(project: &Project, results: &Results) -> String {
    let f = NumberFormat::new(&project.report.units, project.report.decimals);
    let mut s = String::new();
    let line = "=".repeat(78);
    s.push_str(&format!("{}\n{}\n{}\n", line, project.report.title, line));
    s.push_str(&format!("Project      : {}\n", project.title));
    s.push_str(&format!("Project no.  : {}\n", project.project_no));
    s.push_str(&format!("Customer     : {}\n", project.customer));
    s.push_str(&format!("Engineer     : {}\n", project.engineer));
    s.push_str(&format!("Date         : {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M")));
    s.push_str(&format!("Units        : {}\n\n", project.report.units.label()));
    if !project.comment.trim().is_empty() {
        s.push_str(&format!("{}\n\n", project.comment));
    }
    s.push_str("1. CALCULATION BASIS\n");
    s.push_str(&format!(
        "   Eurocode design: {} (text evidenced: D_TXT_RECHNUNG_NACH_EUROCODE)\n   gamma_F = {:.3} (evidenced text: D_TXT_GAMMA_1_5_FACH @ 0x140103ccc)\n   gamma_M steel = {:.3}   gamma_M timber = {:.3}   k_mod = {:.3}\n   Interaction: {}\n   Note: CDbeam.CalcAuslastung subcase formulas and getter defaults not recovered (MECHANICS.md).\n\n",
        if project.calc.eurocode { "yes" } else { "no" },
        project.calc.gamma_f,
        project.calc.gamma_m_steel,
        project.calc.gamma_m_timber,
        project.calc.kmod_timber,
        project.calc.interaction.label()
    ));
    s.push_str("2. MEMBERS\n");
    s.push_str(&format!(
        "   {:<4} {:<22} {:>12} {:>10} {:>10} {:>12} {:>10}\n",
        "No.", "Type", "Length", "I", "E", "M per", "V per"
    ));
    for (i, m) in project.members.iter().enumerate() {
        s.push_str(&format!(
            "   {:<4} {:<22} {:>12} {:>10} {:>10} {:>12} {:>10}\n",
            i + 1,
            truncate(&m.type_name, 22),
            format!("{} {}", f.length(m.length_mm), f.length_unit()),
            format!("{} {}", f.inertia(m.i_cm4), f.inertia_unit()),
            format!("{} {}", f.e_module(m.e_kn_cm2), f.e_unit()),
            format!("{} {}", f.moment(m.perm_m_knm), f.moment_unit()),
            format!("{} {}", f.force(m.perm_v_kn), f.force_unit())
        ));
    }
    s.push('\n');
    s.push_str("3. BEARINGS\n");
    for (i, b) in project.bearings.iter().enumerate() {
        s.push_str(&format!(
            "   {:<4} Joint {:<4} {:<28} x = {} {}\n",
            i + 1,
            b.node + 1,
            b.condition_label(),
            f.length(project.node_x_mm(b.node)),
            f.length_unit()
        ));
    }
    s.push('\n');
    s.push_str("4. LOADINGS\n");
    for (i, l) in project.loads.iter().enumerate() {
        s.push_str(&format!(
            "   {:<4} {:<18} {:<4} {}\n",
            i + 1,
            l.kind.label(),
            if l.active { "on" } else { "off" },
            load_text(l, &f)
        ));
    }
    s.push('\n');
    s.push_str("5. RESULTS\n");
    s.push_str(&format!(
        "   Total length        : {} {}\n   Total vertical load : {} {}\n   Sum of reactions    : {} {}\n   Maximum |M|         : {} {}\n   Maximum |V|         : {} {}\n   Maximum deflection  : {} {}\n   Maximum rotation    : {} rad\n   Maximum utilization : {:.3}\n   Status              : {}\n\n",
        f.length(project.total_length_mm()),
        f.length_unit(),
        f.force(results.total_load_kn),
        f.force_unit(),
        f.force(results.sum_reactions_kn),
        f.force_unit(),
        f.moment(results.max_m),
        f.moment_unit(),
        f.force(results.max_v),
        f.force_unit(),
        f.deflection(results.max_defl),
        f.deflection_unit(),
        f.rotation(results.max_rot),
        results.max_util,
        if results.overloaded { "Overloaded !" } else { "Within capacity" }
    ));
    s.push_str("   MEMBER RESULTS\n");
    s.push_str(&format!(
        "   {:<4} {:>12} {:>12} {:>12} {:>12} {:>10} {:>8}\n",
        "No.", "M max", "M min", "V max", "V min", "Deflection", "Util."
    ));
    for r in &results.members {
        s.push_str(&format!(
            "   {:<4} {:>12} {:>12} {:>12} {:>12} {:>10} {:>8.3}\n",
            r.index + 1,
            f.moment(r.m_max),
            f.moment(r.m_min),
            f.force(r.v_max),
            f.force(r.v_min),
            f.deflection(r.defl_max.abs().max(r.defl_min.abs())),
            r.util
        ));
    }
    s.push('\n');
    s.push_str("   REACTIONS\n");
    s.push_str(&format!(
        "   {:<6} {:>8} {:>14} {:>14}  {}\n",
        "Joint", "x", "Vertical", "Moment", "Bearing"
    ));
    for r in &results.reactions {
        s.push_str(&format!(
            "   {:<6} {:>8} {:>14} {:>14}  {}\n",
            r.node + 1,
            f.length(r.x_mm),
            f.force(r.force_kn),
            f.moment(r.moment_knm),
            r.kind
        ));
    }
    if project.report.include_notes && !project.report.notes.trim().is_empty() {
        s.push_str("\n6. NOTES\n");
        s.push_str(&format!("   {}\n", project.report.notes));
    }
    if project.report.include_signature {
        s.push_str(&format!("\n{}\n", project.report.signature_line));
    }
    s
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n.saturating_sub(1)).collect::<String>() + "."
    }
}

pub enum DiagramKind {
    Moment,
    Shear,
    Deflection,
    Rotation,
}

fn diagram_svg(project: &Project, results: &Results, kind: DiagramKind, f: &NumberFormat) -> String {
    let width = 900.0_f64;
    let height = 210.0_f64;
    let left = 48.0_f64;
    let right = width - 16.0;
    let top = 18.0_f64;
    let bottom = height - 34.0;
    let total = project.total_length_mm().max(1.0);
    let (label, unit, color) = match kind {
        DiagramKind::Moment => ("Bending moment", f.moment_unit(), "#2d6f9f"),
        DiagramKind::Shear => ("Shear force", f.force_unit(), "#a4582a"),
        DiagramKind::Deflection => ("Deflection", f.deflection_unit(), "#2f7d59"),
        DiagramKind::Rotation => ("Rotation", "rad", "#6a4f9f"),
    };
    let mut values: Vec<(f64, f64)> = Vec::new();
    for m in &results.members {
        for s in &m.samples {
            let v = match kind {
                DiagramKind::Moment => s.m_knm,
                DiagramKind::Shear => s.v_kn,
                DiagramKind::Deflection => s.defl_mm,
                DiagramKind::Rotation => s.rot_rad,
            };
            values.push((s.x_mm, v));
        }
    }
    let mut max_v = 0.0_f64;
    let mut min_v = 0.0_f64;
    for (_, v) in &values {
        max_v = max_v.max(*v);
        min_v = min_v.min(*v);
    }
    if (max_v - min_v).abs() < 1e-12 {
        max_v = 1.0;
        min_v = -1.0;
    }
    let pad = (max_v - min_v) * 0.10;
    max_v += pad;
    min_v -= pad;
    let y = |v: f64| -> f64 {
        let t = (v - min_v) / (max_v - min_v);
        bottom - t * (bottom - top)
    };
    let x = |mm: f64| -> f64 { left + (mm / total) * (right - left) };
    let zero_y = y(0.0);

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"{label}\">\n"
    ));
    s.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"#ffffff\"/>\n"
    ));
    // horizontal grid
    for k in 1..4 {
        let yy = top + (bottom - top) * k as f64 / 4.0;
        s.push_str(&format!(
            "<line x1=\"{left:.1}\" y1=\"{yy:.1}\" x2=\"{right:.1}\" y2=\"{yy:.1}\" stroke=\"#e6ebf0\" stroke-width=\"1\"/>\n"
        ));
    }
    s.push_str(&format!(
        "<line x1=\"{left:.1}\" y1=\"{zero_y:.1}\" x2=\"{right:.1}\" y2=\"{zero_y:.1}\" stroke=\"#71808f\" stroke-width=\"1.2\"/>\n"
    ));
    // vertical joint lines
    let mut nx = 0.0;
    for (i, m) in project.members.iter().enumerate() {
        s.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{top:.1}\" x2=\"{:.1}\" y2=\"{bottom:.1}\" stroke=\"#eef1f4\" stroke-width=\"1\"/>\n",
            x(nx),
            x(nx)
        ));
        nx += m.length_mm;
        if i + 1 == project.members.len() {
            s.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{top:.1}\" x2=\"{:.1}\" y2=\"{bottom:.1}\" stroke=\"#eef1f4\" stroke-width=\"1\"/>\n",
                x(nx),
                x(nx)
            ));
        }
    }
    // area + line
    if !values.is_empty() {
        let mut area = String::new();
        area.push_str(&format!("M {:.1} {:.1} ", x(values[0].0), zero_y));
        for (xx, vv) in &values {
            area.push_str(&format!("L {:.1} {:.1} ", x(*xx), y(*vv)));
        }
        area.push_str(&format!("L {:.1} {:.1} Z", x(values.last().unwrap().0), zero_y));
        s.push_str(&format!(
            "<path d=\"{area}\" fill=\"{color}\" fill-opacity=\"0.16\" stroke=\"none\"/>\n"
        ));
        let mut line = String::new();
        for (i, (xx, vv)) in values.iter().enumerate() {
            if i == 0 {
                line.push_str(&format!("M {:.1} {:.1} ", x(*xx), y(*vv)));
            } else {
                line.push_str(&format!("L {:.1} {:.1} ", x(*xx), y(*vv)));
            }
        }
        s.push_str(&format!(
            "<path d=\"{line}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"/>\n"
        ));
    }
    s.push_str(&format!(
        "<text x=\"{left:.1}\" y=\"14\" font-family=\"Segoe UI, sans-serif\" font-size=\"12\" font-weight=\"600\" fill=\"#28313a\">{label}</text>\n"
    ));
    s.push_str(&format!(
        "<text x=\"{right:.1}\" y=\"14\" text-anchor=\"end\" font-family=\"Segoe UI, sans-serif\" font-size=\"10\" fill=\"#6a7480\">[{unit}]</text>\n"
    ));
    s.push_str(&format!(
        "<text x=\"{left:.1}\" y=\"{height_minus:.1}\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#6a7480\">max {}</text>\n",
        f_or(max_v - pad, f, &kind),
        height_minus = height - 10.0
    ));
    s.push_str(&format!(
        "<text x=\"{left:.1}\" y=\"{height_minus2:.1}\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#6a7480\">min {}</text>\n",
        f_or(min_v + pad, f, &kind),
        height_minus2 = height - 24.0
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{height_minus3:.1}\" text-anchor=\"end\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#6a7480\">x total {} {}</text>\n",
        right,
        f.length(total),
        f.length_unit(),
        height_minus3 = height - 10.0
    ));
    s.push_str("</svg>\n");
    s
}

fn f_or(value: f64, f: &NumberFormat, kind: &DiagramKind) -> String {
    match kind {
        DiagramKind::Moment => f.moment(value),
        DiagramKind::Shear => f.force(value),
        DiagramKind::Deflection => f.deflection(value),
        DiagramKind::Rotation => f.rotation(value),
    }
}

fn beam_svg(project: &Project, results: &Results, f: &NumberFormat) -> String {
    let width = 900.0_f64;
    let height = 250.0_f64;
    let left = 44.0_f64;
    let right = width - 20.0;
    let beam_y = 120.0_f64;
    let total = project.total_length_mm().max(1.0);
    let x = |mm: f64| left + (mm / total) * (right - left);
    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"Beam and loadings\">\n"
    ));
    s.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"#ffffff\"/>\n"
    ));
    s.push_str(&format!(
        "<line x1=\"{left:.1}\" y1=\"{beam_y:.1}\" x2=\"{right:.1}\" y2=\"{beam_y:.1}\" stroke=\"#2d3742\" stroke-width=\"7\" stroke-linecap=\"round\"/>\n"
    ));
    // joints and member lengths
    let mut nx = 0.0;
    for i in 0..project.members.len() {
        let x0 = x(nx);
        nx += project.members[i].length_mm;
        let x1 = x(nx);
        s.push_str(&format!(
            "<line x1=\"{x0:.1}\" y1=\"{:.1}\" x2=\"{x0:.1}\" y2=\"{:.1}\" stroke=\"#8c99a6\" stroke-width=\"1\"/>\n",
            beam_y - 12.0,
            beam_y + 18.0
        ));
        if i + 1 == project.members.len() {
            s.push_str(&format!(
                "<line x1=\"{x1:.1}\" y1=\"{:.1}\" x2=\"{x1:.1}\" y2=\"{:.1}\" stroke=\"#8c99a6\" stroke-width=\"1\"/>\n",
                beam_y - 12.0,
                beam_y + 18.0
            ));
        }
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#5c6874\">{} {}</text>\n",
            (x0 + x1) / 2.0,
            beam_y + 34.0,
            f.length(project.members[i].length_mm),
            f.length_unit()
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Segoe UI, sans-serif\" font-size=\"8\" fill=\"#8a96a2\">Member {}</text>\n",
            (x0 + x1) / 2.0,
            beam_y + 46.0,
            i + 1
        ));
    }
    // supports
    for b in &project.bearings {
        let xx = x(project.node_x_mm(b.node));
        match b.kind {
            BearingKind::None => {}
            BearingKind::Fixed => {
                s.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"10\" height=\"34\" fill=\"#3d4b59\"/>\n",
                    xx - 5.0,
                    beam_y + 8.0
                ));
                for k in 0..4 {
                    s.push_str(&format!(
                        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#3d4b59\" stroke-width=\"1.5\"/>\n",
                        xx - 12.0 - k as f64 * 4.0,
                        beam_y + 12.0 + k as f64 * 6.0,
                        xx - 2.0 - k as f64 * 4.0,
                        beam_y + 18.0 + k as f64 * 6.0
                    ));
                }
            }
            BearingKind::Sliding | BearingKind::Hinge => {
                s.push_str(&format!(
                    "<path d=\"M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z\" fill=\"none\" stroke=\"#2d3742\" stroke-width=\"1.8\"/>\n",
                    xx,
                    beam_y + 4.0,
                    xx - 12.0,
                    beam_y + 30.0,
                    xx + 12.0,
                    beam_y + 30.0
                ));
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3.2\" fill=\"#ffffff\" stroke=\"#2d3742\" stroke-width=\"1.4\"/>\n",
                    xx - 6.0,
                    beam_y + 34.0
                ));
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3.2\" fill=\"#ffffff\" stroke=\"#2d3742\" stroke-width=\"1.4\"/>\n",
                    xx + 6.0,
                    beam_y + 34.0
                ));
            }
            BearingKind::Elastic => {
                s.push_str(&format!(
                    "<path d=\"M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1}\" fill=\"none\" stroke=\"#2d6f9f\" stroke-width=\"1.8\"/>\n",
                    xx,
                    beam_y + 4.0,
                    xx - 8.0,
                    beam_y + 12.0,
                    xx + 8.0,
                    beam_y + 20.0,
                    xx - 8.0,
                    beam_y + 28.0,
                    xx + 8.0,
                    beam_y + 36.0,
                    xx,
                    beam_y + 42.0
                ));
            }
        }
        if b.is_hinge() {
            s.push_str(&format!(
                "<circle cx=\"{xx:.1}\" cy=\"{beam_y:.1}\" r=\"6\" fill=\"#ffffff\" stroke=\"#2d3742\" stroke-width=\"1.6\"/>\n"
            ));
        }
    }
    // loads
    for l in &project.loads {
        if !l.active {
            continue;
        }
        match l.kind {
            LoadKind::Point => {
                let xx = x(l.c);
                s.push_str(&format!(
                    "<line x1=\"{xx:.1}\" y1=\"30\" x2=\"{xx:.1}\" y2=\"{:.1}\" stroke=\"#b3412d\" stroke-width=\"2\" marker-end=\"url(#arrow)\"/>\n",
                    beam_y - 6.0
                ));
                s.push_str(&format!(
                    "<text x=\"{xx:.1}\" y=\"24\" text-anchor=\"middle\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#b3412d\">{} {}</text>\n",
                    f.force(l.b),
                    f.force_unit()
                ));
            }
            LoadKind::Moment => {
                let xx = x(l.c);
                s.push_str(&format!(
                    "<path d=\"M {:.1} 50 A 18 18 0 1 1 {:.1} 50\" fill=\"none\" stroke=\"#9a6a1f\" stroke-width=\"2\"/>\n",
                    xx - 18.0,
                    xx + 18.0
                ));
                s.push_str(&format!(
                    "<text x=\"{xx:.1}\" y=\"42\" text-anchor=\"middle\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#9a6a1f\">{} {}</text>\n",
                    f.moment(l.a),
                    f.moment_unit()
                ));
            }
            LoadKind::Trapezoid | LoadKind::ConcretePressure => {
                let x0 = x(l.c);
                let x1 = x((l.c + l.d).min(total));
                let (q0, q1) = if matches!(l.kind, LoadKind::ConcretePressure) {
                    (l.a * l.width_m(), l.b * l.width_m())
                } else {
                    (l.a, l.b)
                };
                let maxq = q0.abs().max(q1.abs()).max(1.0);
                let y0 = beam_y - 18.0 - (q0.abs() / maxq) * 36.0;
                let y1 = beam_y - 18.0 - (q1.abs() / maxq) * 36.0;
                let mut d = format!("M {x0:.1} {:.1} L {x1:.1} {:.1} L {x1:.1} {:.1} L {x0:.1} {:.1} Z", beam_y - 6.0, beam_y - 6.0, y1, y0);
                s.push_str(&format!(
                    "<path d=\"{d}\" fill=\"#e8b44a\" fill-opacity=\"0.25\" stroke=\"#b8862b\" stroke-width=\"1.3\"/>\n"
                ));
                d.clear();
                let n = 12;
                for k in 0..=n {
                    let t = k as f64 / n as f64;
                    let xx = x0 + (x1 - x0) * t;
                    let yy = beam_y - 8.0;
                    s.push_str(&format!(
                        "<line x1=\"{xx:.1}\" y1=\"{yy:.1}\" x2=\"{xx:.1}\" y2=\"{:.1}\" stroke=\"#b8862b\" stroke-width=\"1\"/>\n",
                        beam_y - 14.0 - ((q0 + (q1 - q0) * t).abs() / maxq) * 34.0
                    ));
                }
                s.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#8a621c\">{} / {} {}</text>\n",
                    (x0 + x1) / 2.0,
                    beam_y - 54.0,
                    f.line_load(q0),
                    f.line_load(q1),
                    f.line_load_unit()
                ));
            }
        }
    }
    // results annotation
    s.push_str(&format!(
        "<text x=\"{left:.1}\" y=\"222\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#5c6874\">max |M| {} {}</text>\n",
        f.moment(results.max_m),
        f.moment_unit()
    ));
    s.push_str(&format!(
        "<text x=\"{left:.1}\" y=\"236\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#5c6874\">max |V| {} {}</text>\n",
        f.force(results.max_v),
        f.force_unit()
    ));
    s.push_str(&format!(
        "<text x=\"{right:.1}\" y=\"222\" text-anchor=\"end\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"#5c6874\">max deflection {} {}</text>\n",
        f.deflection(results.max_defl),
        f.deflection_unit()
    ));
    s.push_str(&format!(
        "<text x=\"{right:.1}\" y=\"236\" text-anchor=\"end\" font-family=\"Segoe UI, sans-serif\" font-size=\"9\" fill=\"{} \">{}</text>\n",
        if results.overloaded { "#a42323" } else { "#176b3a" },
        if results.overloaded { "Overloaded !" } else { "Within capacity" }
    ));
    s.push_str("<defs><marker id=\"arrow\" markerWidth=\"8\" markerHeight=\"8\" refX=\"4\" refY=\"4\" orient=\"auto\"><path d=\"M0,0 L8,4 L0,8 Z\" fill=\"#b3412d\"/></marker></defs>\n");
    s.push_str("</svg>\n");
    s
}

pub struct PreviewBlock {
    pub text: String,
    pub kind: PreviewKind,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PreviewKind {
    Title,
    Subtitle,
    Heading,
    Body,
    TableHeader,
    TableRow,
    Note,
    Footer,
}

pub fn preview_blocks(project: &Project, results: &Results) -> Vec<PreviewBlock> {
    let o = &project.report;
    let f = NumberFormat::new(&o.units, o.decimals);
    let mut v = Vec::new();
    v.push(PreviewBlock { text: o.title.clone(), kind: PreviewKind::Title });
    if !o.subtitle.is_empty() {
        v.push(PreviewBlock { text: o.subtitle.clone(), kind: PreviewKind::Subtitle });
    }
    v.push(PreviewBlock {
        text: format!("{}  |  {}  |  {}", project.title, project.project_no, project.customer),
        kind: PreviewKind::Body,
    });
    v.push(PreviewBlock {
        text: format!("{}  |  {}", project.engineer, chrono::Local::now().format("%Y-%m-%d")),
        kind: PreviewKind::Body,
    });
    v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
    v.push(PreviewBlock { text: "1. Calculation basis".to_string(), kind: PreviewKind::Heading });
    v.push(PreviewBlock {
        text: format!(
            "{}  |  gamma_F {:.2}  |  gamma_M steel {:.2}  |  gamma_M timber {:.2}  |  k_mod {:.2}",
            if project.calc.eurocode { "Eurocode" } else { "Permissible loads" },
            project.calc.gamma_f,
            project.calc.gamma_m_steel,
            project.calc.gamma_m_timber,
            project.calc.kmod_timber
        ),
        kind: PreviewKind::Body,
    });
    v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
    if o.include_inputs {
        if o.include_members {
            v.push(PreviewBlock { text: "2. Members".to_string(), kind: PreviewKind::Heading });
            v.push(PreviewBlock {
                text: format!(
                    "{:<4} {:<18} {:>10} {:>10} {:>10} {:>10}",
                    "No.", "Type", "Length", "I", "E", "M per"
                ),
                kind: PreviewKind::TableHeader,
            });
            for (i, m) in project.members.iter().enumerate() {
                v.push(PreviewBlock {
                    text: format!(
                        "{:<4} {:<18} {:>10} {:>10} {:>10} {:>10}",
                        i + 1,
                        truncate(&m.type_name, 18),
                        f.length(m.length_mm),
                        f.inertia(m.i_cm4),
                        f.e_module(m.e_kn_cm2),
                        f.moment(m.perm_m_knm)
                    ),
                    kind: PreviewKind::TableRow,
                });
            }
            v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
        }
        if o.include_bearings {
            v.push(PreviewBlock { text: "Bearings".to_string(), kind: PreviewKind::Heading });
            for b in &project.bearings {
                v.push(PreviewBlock {
                    text: format!(
                        "Joint {:<3}  {:<28} x = {} {}",
                        b.node + 1,
                        truncate(&b.condition_label(), 28),
                        f.length(project.node_x_mm(b.node)),
                        f.length_unit()
                    ),
                    kind: PreviewKind::TableRow,
                });
            }
            v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
        }
        if o.include_loads {
            v.push(PreviewBlock { text: "Loadings".to_string(), kind: PreviewKind::Heading });
            for l in &project.loads {
                v.push(PreviewBlock {
                    text: format!(
                        "{}  {:<16} {}",
                        if l.active { "on " } else { "off" },
                        l.kind.label(),
                        load_text(l, &f)
                    ),
                    kind: PreviewKind::TableRow,
                });
            }
            v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
        }
    }
    if o.include_results {
        v.push(PreviewBlock { text: "3. Results".to_string(), kind: PreviewKind::Heading });
        v.push(PreviewBlock {
            text: format!(
                "Total load {} {}   |   Sum reactions {} {}   |   max |M| {} {}   |   max |V| {} {}",
                f.force(results.total_load_kn),
                f.force_unit(),
                f.force(results.sum_reactions_kn),
                f.force_unit(),
                f.moment(results.max_m),
                f.moment_unit(),
                f.force(results.max_v),
                f.force_unit()
            ),
            kind: PreviewKind::Body,
        });
        v.push(PreviewBlock {
            text: format!(
                "max deflection {} {}   |   max rotation {} rad   |   utilization {:.2}",
                f.deflection(results.max_defl),
                f.deflection_unit(),
                f.rotation(results.max_rot),
                results.max_util
            ),
            kind: PreviewKind::Body,
        });
        v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
        v.push(PreviewBlock {
            text: format!(
                "{:<4} {:>10} {:>10} {:>10} {:>10} {:>8}",
                "No.", "M max", "M min", "V max", "V min", "Util."
            ),
            kind: PreviewKind::TableHeader,
        });
        for r in &results.members {
            v.push(PreviewBlock {
                text: format!(
                    "{:<4} {:>10} {:>10} {:>10} {:>10} {:>8.2}",
                    r.index + 1,
                    f.moment(r.m_max),
                    f.moment(r.m_min),
                    f.force(r.v_max),
                    f.force(r.v_min),
                    r.util
                ),
                kind: PreviewKind::TableRow,
            });
        }
        v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
        v.push(PreviewBlock {
            text: format!(
                "{}   |   {}",
                results.summary_line(),
                if results.overloaded { "Overloaded !" } else { "All checks satisfied" }
            ),
            kind: if results.overloaded { PreviewKind::Note } else { PreviewKind::Body },
        });
    }
    if o.include_checks {
        v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
        v.push(PreviewBlock { text: "4. Checks".to_string(), kind: PreviewKind::Heading });
        v.push(PreviewBlock {
            text: format!(
                "Governing member {}   |   utilization {:.3} / 1.000   |   {}",
                results.max_util_member + 1,
                results.max_util,
                if results.overloaded { "Overloaded !" } else { "OK" }
            ),
            kind: PreviewKind::Body,
        });
        if project.calc.interaction == crate::model::Interaction::None {
            v.push(PreviewBlock { text: "no M-V interaction".to_string(), kind: PreviewKind::Body });
        }
    }
    if o.include_notes && !o.notes.trim().is_empty() {
        v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
        v.push(PreviewBlock { text: "5. Notes".to_string(), kind: PreviewKind::Heading });
        v.push(PreviewBlock { text: o.notes.clone(), kind: PreviewKind::Note });
    }
    if o.include_signature {
        v.push(PreviewBlock { text: String::new(), kind: PreviewKind::Body });
        v.push(PreviewBlock { text: o.signature_line.clone(), kind: PreviewKind::Footer });
    }
    if o.include_header {
        v.push(PreviewBlock {
            text: format!(
                "{}   |   {}   |   {}",
                token_text(project, &o.footer_left, 1, 1),
                token_text(project, &o.footer_center, 1, 1),
                token_text(project, &o.footer_right, 1, 1)
            ),
            kind: PreviewKind::Footer,
        });
    }
    v
}

pub fn member_status(r: &MemberResult) -> &'static str {
    if r.overloaded {
        "Overloaded !"
    } else {
        "OK"
    }
}

use crate::model::{
    BearingKind, Interaction, Load, LoadKind, Material, Member, Project,
};

#[derive(Clone, Debug)]
pub struct Sample {
    pub x_mm: f64,
    pub m_knm: f64,
    pub v_kn: f64,
    pub defl_mm: f64,
    pub rot_rad: f64,
}

#[derive(Clone, Debug)]
pub struct MemberResult {
    pub index: usize,
    pub x_start_mm: f64,
    pub x_end_mm: f64,
    pub m_max: f64,
    pub m_min: f64,
    pub m_max_pos: f64,
    pub m_min_pos: f64,
    pub v_max: f64,
    pub v_min: f64,
    pub v_max_pos: f64,
    pub v_min_pos: f64,
    pub defl_max: f64,
    pub defl_min: f64,
    pub defl_max_pos: f64,
    pub rot_max: f64,
    pub rot_min: f64,
    pub util_m: f64,
    pub util_v: f64,
    pub util: f64,
    pub overloaded: bool,
    pub samples: Vec<Sample>,
}

#[derive(Clone, Debug)]
pub struct Reaction {
    pub node: usize,
    pub x_mm: f64,
    pub force_kn: f64,
    pub moment_knm: f64,
    pub kind: String,
}

#[derive(Clone, Debug)]
pub struct NodeResult {
    pub node: usize,
    pub x_mm: f64,
    pub defl_mm: f64,
    pub rot_rad: f64,
}

#[derive(Clone, Debug)]
pub struct Results {
    pub members: Vec<MemberResult>,
    pub nodes: Vec<NodeResult>,
    pub reactions: Vec<Reaction>,
    pub total_load_kn: f64,
    pub sum_reactions_kn: f64,
    pub max_m: f64,
    pub max_v: f64,
    pub max_defl: f64,
    pub max_rot: f64,
    pub max_util: f64,
    pub max_util_member: usize,
    pub overloaded: bool,
}

impl Results {
    pub fn summary_line(&self) -> String {
        if self.overloaded {
            format!(
                "Overloaded - maximum utilization {:.2} ({})",
                self.max_util,
                self.max_util_member + 1
            )
        } else {
            format!("Within capacity - maximum utilization {:.2}", self.max_util)
        }
    }
}

#[derive(Clone, Copy)]
struct LoadStation {
    x_local: f64,
    value: f64,
    is_moment: bool,
}

pub fn solve(project: &Project) -> Result<Results, String> {
    let issues = project.validate();
    if !issues.is_empty() {
        return Err(issues.join(" "));
    }
    let n_members = project.members.len();
    if n_members == 0 {
        return Err("Beam has no members.".to_string());
    }
    let n_nodes = n_members + 1;

    let mut hinge = vec![false; n_nodes];
    let mut fixed_rotation = vec![false; n_nodes];
    let mut vertical = vec![false; n_nodes];
    let mut spring_kn_m = vec![0.0_f64; n_nodes];
    let mut bearing_kind: Vec<String> = vec!["Free".to_string(); n_nodes];

    for b in &project.bearings {
        if b.node >= n_nodes {
            continue;
        }
        if b.is_hinge() {
            hinge[b.node] = true;
        }
        if b.fixed_rotation() {
            fixed_rotation[b.node] = true;
        }
        if b.vertical() {
            vertical[b.node] = true;
            bearing_kind[b.node] = b.kind.label().to_string();
            if matches!(b.kind, BearingKind::Elastic) {
                let k = b.spring_kn_m();
                if k <= 0.0 {
                    return Err("Elastic bearing has no spring stiffness.".to_string());
                }
                spring_kn_m[b.node] += k;
            }
        }
    }

    // Vertical displacement DOF for every joint. Rotations are shared unless a
    // moment hinge splits the joint into a left and a right rotation.
    let mut ndof = n_nodes;
    let mut rot = vec![[0usize; 2]; n_nodes];
    for i in 0..n_nodes {
        if hinge[i] {
            rot[i] = [ndof, ndof + 1];
            ndof += 2;
        } else {
            rot[i] = [ndof, ndof];
            ndof += 1;
        }
    }

    let mut k_full = vec![vec![0.0_f64; ndof]; ndof];
    let mut f_full = vec![0.0_f64; ndof];
    let mut element_k: Vec<[[f64; 4]; 4]> = Vec::with_capacity(n_members);
    let mut element_feq: Vec<[f64; 4]> = Vec::with_capacity(n_members);
    let mut element_dofs: Vec<[usize; 4]> = Vec::with_capacity(n_members);

    for (i, member) in project.members.iter().enumerate() {
        let l = member.length_m();
        if l <= 0.0 {
            return Err("Member length not positive!".to_string());
        }
        let ei = member.ei_knm2();
        if ei <= 0.0 {
            return Err("Member stiffness is zero.".to_string());
        }
        let k = beam_stiffness(ei, l);
        let mut feq = [0.0_f64; 4];

        let x0 = project.member_start_mm(i) / 1000.0;
        let x1 = x0 + l;

        // Distributed loads that overlap this member.
        for load in &project.loads {
            if !load.active {
                continue;
            }
            match load.kind {
                LoadKind::Trapezoid | LoadKind::ConcretePressure => {
                    let start = load.c / 1000.0;
                    let len = load.d / 1000.0;
                    if len <= 0.0 {
                        continue;
                    }
                    let end = start + len;
                    let a = start.max(x0);
                    let b = end.min(x1);
                    if b > a + 1e-9 {
                        let width = if matches!(load.kind, LoadKind::ConcretePressure) {
                            load.width_m()
                        } else {
                            1.0
                        };
                        add_distributed(
                            &mut feq,
                            a - x0,
                            b - x0,
                            l,
                            |s| q_at(load, start, len, s) * width,
                        );
                    }
                }
                _ => {}
            }
        }

        if project.calc.include_self_weight && member.self_weight_kn_m > 0.0 {
            add_distributed(&mut feq, 0.0, l, l, |_| member.self_weight_kn_m);
        }

        // Point loads and concentrated moments.
        for load in &project.loads {
            if !load.active {
                continue;
            }
            match load.kind {
                LoadKind::Point => {
                    let p = load.b;
                    if p.abs() < 1e-12 {
                        continue;
                    }
                    let x = load.c / 1000.0;
                    if on_element(x, x0, x1, i, n_members) {
                        let t = ((x - x0) / l).clamp(0.0, 1.0);
                        add_point(&mut feq, t, l, -p);
                    }
                }
                LoadKind::Moment => {
                    if load.a.abs() < 1e-12 {
                        continue;
                    }
                    let x = load.c / 1000.0;
                    if on_element(x, x0, x1, i, n_members) {
                        let t = ((x - x0) / l).clamp(0.0, 1.0);
                        add_moment(&mut feq, t, l, load.a);
                    }
                }
                _ => {}
            }
        }

        let dofs = [i, rot[i][1], i + 1, rot[i + 1][0]];
        for r in 0..4 {
            f_full[dofs[r]] += feq[r];
            for c in 0..4 {
                k_full[dofs[r]][dofs[c]] += k[r][c];
            }
        }
        element_k.push(k);
        element_feq.push(feq);
        element_dofs.push(dofs);
    }

    let mut fixed = vec![false; ndof];
    for i in 0..n_nodes {
        if vertical[i] && spring_kn_m[i] <= 0.0 {
            fixed[i] = true;
        }
        if spring_kn_m[i] > 0.0 {
            k_full[i][i] += spring_kn_m[i];
        }
        if fixed_rotation[i] {
            fixed[rot[i][0]] = true;
            fixed[rot[i][1]] = true;
        }
    }

    let mut free = Vec::new();
    for i in 0..ndof {
        if !fixed[i] {
            free.push(i);
        }
    }
    let mut u = vec![0.0_f64; ndof];
    if !free.is_empty() {
        let mut kff = vec![vec![0.0_f64; free.len()]; free.len()];
        let mut ff = vec![0.0_f64; free.len()];
        for (a, &ia) in free.iter().enumerate() {
            ff[a] = f_full[ia];
            for (b, &ib) in free.iter().enumerate() {
                kff[a][b] = k_full[ia][ib];
            }
        }
        let x = solve_linear(kff, ff).ok_or_else(|| {
            "Beam statical underdetermined! Check bearing conditions.".to_string()
        })?;
        for (a, &ia) in free.iter().enumerate() {
            u[ia] = x[a];
        }
    }

    let mut reactions = Vec::new();
    let mut sum_reactions = 0.0;
    for i in 0..n_nodes {
        let x_mm = project.node_x_mm(i);
        let mut force = 0.0;
        let mut moment = 0.0;
        let mut kind = bearing_kind[i].clone();
        if spring_kn_m[i] > 0.0 {
            force = -spring_kn_m[i] * u[i];
            kind = "Elastic bearing".to_string();
        } else if fixed[i] {
            let mut s = 0.0;
            for j in 0..ndof {
                s += k_full[i][j] * u[j];
            }
            force = s - f_full[i];
        }
        if fixed_rotation[i] {
            let r0 = rot[i][0];
            let r1 = rot[i][1];
            for r in [r0, r1] {
                let mut s = 0.0;
                for j in 0..ndof {
                    s += k_full[r][j] * u[j];
                }
                moment += s - f_full[r];
            }
        }
        if vertical[i] || fixed_rotation[i] || spring_kn_m[i] > 0.0 {
            sum_reactions += force;
            reactions.push(Reaction {
                node: i,
                x_mm,
                force_kn: force,
                moment_knm: moment,
                kind,
            });
        }
    }

    let mut nodes = Vec::new();
    for i in 0..n_nodes {
        let rotation = if i == 0 {
            u[rot[i][1]]
        } else {
            u[rot[i][0]]
        };
        nodes.push(NodeResult {
            node: i,
            x_mm: project.node_x_mm(i),
            defl_mm: -u[i] * 1000.0,
            rot_rad: rotation,
        });
    }

    let mut member_results = Vec::new();
    let mut max_m = 0.0_f64;
    let mut max_v = 0.0_f64;
    let mut max_defl = 0.0_f64;
    let mut max_rot = 0.0_f64;
    let mut max_util = 0.0_f64;
    let mut max_util_member = 0usize;

    for (i, member) in project.members.iter().enumerate() {
        let l = member.length_m();
        let x0 = project.member_start_mm(i) / 1000.0;
        let dofs = element_dofs[i];
        let ue = [u[dofs[0]], u[dofs[1]], u[dofs[2]], u[dofs[3]]];
        let k = element_k[i];
        let feq = element_feq[i];
        let mut p = [0.0_f64; 4];
        for r in 0..4 {
            for c in 0..4 {
                p[r] += k[r][c] * ue[c];
            }
            p[r] -= feq[r];
        }

        let mut stations: Vec<f64> = (0..=40).map(|j| l * j as f64 / 40.0).collect();
        let mut point_loads: Vec<LoadStation> = Vec::new();
        for load in &project.loads {
            if !load.active {
                continue;
            }
            match load.kind {
                LoadKind::Point => {
                    let xg = load.c / 1000.0;
                    if on_element(xg, x0, x0 + l, i, n_members) {
                        let x = (xg - x0).clamp(0.0, l);
                        stations.push(x);
                        point_loads.push(LoadStation {
                            x_local: x,
                            value: load.b,
                            is_moment: false,
                        });
                    }
                }
                LoadKind::Moment => {
                    let xg = load.c / 1000.0;
                    if on_element(xg, x0, x0 + l, i, n_members) {
                        let x = (xg - x0).clamp(0.0, l);
                        stations.push(x);
                        point_loads.push(LoadStation {
                            x_local: x,
                            value: load.a,
                            is_moment: true,
                        });
                    }
                }
                _ => {}
            }
        }
        stations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        stations.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

        let q_here = |s_local: f64| -> f64 {
            let s_global = x0 + s_local;
            let mut q = 0.0;
            for load in &project.loads {
                if !load.active {
                    continue;
                }
                match load.kind {
                    LoadKind::Trapezoid | LoadKind::ConcretePressure => {
                        let start = load.c / 1000.0;
                        let len = load.d / 1000.0;
                        if len > 0.0 && s_global >= start - 1e-9 && s_global <= start + len + 1e-9 {
                            let width = if matches!(load.kind, LoadKind::ConcretePressure) {
                                load.width_m()
                            } else {
                                1.0
                            };
                            q += q_at(load, start, len, s_global) * width;
                        }
                    }
                    _ => {}
                }
            }
            if project.calc.include_self_weight {
                q += member.self_weight_kn_m;
            }
            q
        };

        let mut v = p[0];
        let mut m = -p[1];
        // Jumps exactly at the left end.
        for st in &point_loads {
            if st.x_local.abs() < 1e-9 {
                if st.is_moment {
                    m -= st.value;
                } else {
                    v -= st.value;
                }
            }
        }

        let mut samples = Vec::with_capacity(stations.len() + 1);
        let mut sample_at = |x: f64,
                             v: f64,
                             m: f64,
                             ue: &[f64; 4],
                             l: f64,
                             out: &mut Vec<Sample>| {
            let t = (x / l).clamp(0.0, 1.0);
            let (n1, n2, n3, n4) = shape_functions(t, l);
            let displacement = n1 * ue[0] + n2 * ue[1] + n3 * ue[2] + n4 * ue[3];
            let (d1, d2, d3, d4) = shape_derivatives(t, l);
            let rotation = d1 * ue[0] + d2 * ue[1] + d3 * ue[2] + d4 * ue[3];
            out.push(Sample {
                x_mm: (x0 + x) * 1000.0,
                m_knm: m,
                v_kn: v,
                defl_mm: -displacement * 1000.0,
                rot_rad: rotation,
            });
        };

        sample_at(stations[0], v, m, &ue, l, &mut samples);
        for w in stations.windows(2) {
            let s0 = w[0];
            let s1 = w[1];
            let ds = s1 - s0;
            if ds <= 0.0 {
                continue;
            }
            let v0 = v;
            let q0 = q_here(s0);
            let q1 = q_here(s1);
            v -= 0.5 * (q0 + q1) * ds;
            m += 0.5 * (v0 + v) * ds;
            sample_at(s1, v, m, &ue, l, &mut samples);
            for st in &point_loads {
                if (st.x_local - s1).abs() < 1e-8 {
                    if st.is_moment {
                        m -= st.value;
                    } else {
                        v -= st.value;
                    }
                }
            }
        }
        // Add a sample just after the right end if a jump was applied there.
        if let Some(last) = samples.last() {
            let mut vv = last.v_kn;
            let mut mm = last.m_knm;
            let mut changed = false;
            for st in &point_loads {
                if (st.x_local - l).abs() < 1e-8 {
                    if st.is_moment {
                        mm -= st.value;
                    } else {
                        vv -= st.value;
                    }
                    changed = true;
                }
            }
            if changed {
                let mut extra = Vec::new();
                sample_at(l, vv, mm, &ue, l, &mut extra);
                samples.push(extra.remove(0));
            }
        }

        let mut m_max = f64::NEG_INFINITY;
        let mut m_min = f64::INFINITY;
        let mut m_max_pos = 0.0;
        let mut m_min_pos = 0.0;
        let mut v_max = f64::NEG_INFINITY;
        let mut v_min = f64::INFINITY;
        let mut v_max_pos = 0.0;
        let mut v_min_pos = 0.0;
        let mut defl_max = f64::NEG_INFINITY;
        let mut defl_min = f64::INFINITY;
        let mut defl_max_pos = 0.0;
        let mut rot_max = f64::NEG_INFINITY;
        let mut rot_min = f64::INFINITY;
        for s in &samples {
            if s.m_knm > m_max {
                m_max = s.m_knm;
                m_max_pos = s.x_mm;
            }
            if s.m_knm < m_min {
                m_min = s.m_knm;
                m_min_pos = s.x_mm;
            }
            if s.v_kn > v_max {
                v_max = s.v_kn;
                v_max_pos = s.x_mm;
            }
            if s.v_kn < v_min {
                v_min = s.v_kn;
                v_min_pos = s.x_mm;
            }
            if s.defl_mm > defl_max {
                defl_max = s.defl_mm;
                defl_max_pos = s.x_mm;
            }
            if s.defl_mm < defl_min {
                defl_min = s.defl_mm;
            }
            rot_max = rot_max.max(s.rot_rad);
            rot_min = rot_min.min(s.rot_rad);
        }

        let m_abs = m_max.abs().max(m_min.abs());
        let v_abs = v_max.abs().max(v_min.abs());
        // Evidence-aligned: utilization/capacity in original is split by calc-code into
        // CDbeam.CalcAuslastungSub1to6/7/8/9/Default/Sub100andMore (iAuslastungsCalcCode);
        // exact subcase formulas/thresholds not recovered (MECHANICS.md). This rebuilt app
        // uses a generic M/V utilization with optional interaction as an explicit approximation.
        let util_m = utilization(m_abs, member.perm_m_knm, member.material.clone(), project);
        let util_v = utilization(v_abs, member.perm_v_kn, member.material.clone(), project);
        let util = match project.calc.interaction {
            Interaction::None => util_m.max(util_v),
            Interaction::Linear => util_m + util_v,
            Interaction::Sqrt => (util_m * util_m + util_v * util_v).sqrt(),
        };

        max_m = max_m.max(m_abs);
        max_v = max_v.max(v_abs);
        max_defl = max_defl.max(defl_max);
        max_rot = max_rot.max(rot_max.abs().max(rot_min.abs()));
        if util > max_util {
            max_util = util;
            max_util_member = i;
        }

        member_results.push(MemberResult {
            index: i,
            x_start_mm: x0 * 1000.0,
            x_end_mm: (x0 + l) * 1000.0,
            m_max,
            m_min,
            m_max_pos,
            m_min_pos,
            v_max,
            v_min,
            v_max_pos,
            v_min_pos,
            defl_max,
            defl_min,
            defl_max_pos,
            rot_max,
            rot_min,
            util_m,
            util_v,
            util,
            // Overload concept evidenced by D_TXT_UEBERLASTUNG/OnWriteBelastungUeberlastungSub;
            // numeric criterion aligned as utilization > 1.0 (original threshold unknown).
            overloaded: util > 1.0 + 1e-9,
            samples,
        });
    }

    let total_load = total_vertical_load(project);
    Ok(Results {
        members: member_results,
        nodes,
        reactions,
        total_load_kn: total_load,
        sum_reactions_kn: sum_reactions,
        max_m,
        max_v,
        max_defl,
        max_rot,
        max_util,
        max_util_member,
        overloaded: max_util > 1.0 + 1e-9, // see MECHANICS.md (D_TXT_UEBERLASTUNG)
    })
}

fn q_at(load: &Load, start: f64, len: f64, s: f64) -> f64 {
    let t = ((s - start) / len).clamp(0.0, 1.0);
    load.a + (load.b - load.a) * t
}

fn on_element(x: f64, x0: f64, x1: f64, index: usize, count: usize) -> bool {
    let eps = 1e-9;
    if index + 1 == count {
        x >= x0 - eps && x <= x1 + eps
    } else {
        x >= x0 - eps && x < x1 - eps
    }
}

fn total_vertical_load(project: &Project) -> f64 {
    let mut total = 0.0;
    for load in &project.loads {
        if !load.active {
            continue;
        }
        match load.kind {
            LoadKind::Point => total += load.b,
            LoadKind::Trapezoid => total += 0.5 * (load.a + load.b) * (load.d / 1000.0),
            LoadKind::ConcretePressure => {
                total += 0.5 * (load.a + load.b) * load.width_m() * (load.d / 1000.0)
            }
            LoadKind::Moment => {}
        }
    }
    if project.calc.include_self_weight {
        total += project
            .members
            .iter()
            .map(|m| m.self_weight_kn_m * m.length_m())
            .sum::<f64>();
    }
    total
}

fn utilization(load: f64, permissible: f64, material: Material, project: &Project) -> f64 {
    if permissible <= 0.0 {
        return if load.abs() > 1e-9 { 999.0 } else { 0.0 };
    }
    if !project.calc.eurocode {
        return load / permissible;
    }
    let gamma_f = if project.calc.actual_load_gamma {
        1.0
    } else {
        project.calc.gamma_f
    };
    let design_load = load * gamma_f;
    let resistance = match material {
        Material::Timber => permissible * project.calc.kmod_timber / project.calc.gamma_m_timber,
        Material::Steel => permissible / project.calc.gamma_m_steel,
        Material::Aluminium => permissible / project.calc.gamma_m_steel,
        Material::Other => permissible,
    };
    if resistance <= 0.0 {
        return 999.0;
    }
    design_load / resistance
}

fn beam_stiffness(ei: f64, l: f64) -> [[f64; 4]; 4] {
    let c = ei / (l * l * l);
    [
        [12.0 * c, 6.0 * l * c, -12.0 * c, 6.0 * l * c],
        [6.0 * l * c, 4.0 * l * l * c, -6.0 * l * c, 2.0 * l * l * c],
        [-12.0 * c, -6.0 * l * c, 12.0 * c, -6.0 * l * c],
        [6.0 * l * c, 2.0 * l * l * c, -6.0 * l * c, 4.0 * l * l * c],
    ]
}

fn add_distributed<F: Fn(f64) -> f64>(
    feq: &mut [f64; 4],
    a: f64,
    b: f64,
    l: f64,
    q: F,
) {
    if b <= a {
        return;
    }
    const X: [f64; 4] = [-0.861_136_311_594_052_6, -0.339_981_043_584_856_3, 0.339_981_043_584_856_3, 0.861_136_311_594_052_6];
    const W: [f64; 4] = [0.347_854_845_137_453_9, 0.652_145_154_862_546_1, 0.652_145_154_862_546_1, 0.347_854_845_137_453_9];
    for i in 0..4 {
        let s = 0.5 * (a + b) + 0.5 * (b - a) * X[i];
        let ds = 0.5 * (b - a) * W[i];
        let t = (s / l).clamp(0.0, 1.0);
        let (n1, n2, n3, n4) = shape_functions(t, l);
        let force = q(s);
        feq[0] -= force * n1 * ds;
        feq[1] -= force * n2 * ds;
        feq[2] -= force * n3 * ds;
        feq[3] -= force * n4 * ds;
    }
}

fn add_point(feq: &mut [f64; 4], t: f64, l: f64, value: f64) {
    let (n1, n2, n3, n4) = shape_functions(t, l);
    feq[0] += value * n1;
    feq[1] += value * n2;
    feq[2] += value * n3;
    feq[3] += value * n4;
}

fn add_moment(feq: &mut [f64; 4], t: f64, l: f64, value: f64) {
    let (d1, d2, d3, d4) = shape_derivatives(t, l);
    feq[0] += value * d1;
    feq[1] += value * d2;
    feq[2] += value * d3;
    feq[3] += value * d4;
}

fn shape_functions(t: f64, l: f64) -> (f64, f64, f64, f64) {
    let t2 = t * t;
    let t3 = t2 * t;
    (
        1.0 - 3.0 * t2 + 2.0 * t3,
        l * (t - 2.0 * t2 + t3),
        3.0 * t2 - 2.0 * t3,
        l * (-t2 + t3),
    )
}

fn shape_derivatives(t: f64, l: f64) -> (f64, f64, f64, f64) {
    let t2 = t * t;
    (
        (-6.0 * t + 6.0 * t2) / l,
        1.0 - 4.0 * t + 3.0 * t2,
        (6.0 * t - 6.0 * t2) / l,
        -2.0 * t + 3.0 * t2,
    )
}

fn solve_linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    if n == 0 {
        return Some(Vec::new());
    }
    for col in 0..n {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..n {
            let v = a[r][col].abs();
            if v > best {
                best = v;
                pivot = r;
            }
        }
        if best < 1e-10 {
            return None;
        }
        if pivot != col {
            a.swap(col, pivot);
            b.swap(col, pivot);
        }
        let piv = a[col][col];
        for r in (col + 1)..n {
            let factor = a[r][col] / piv;
            if factor.abs() < 1e-18 {
                continue;
            }
            for c in col..n {
                a[r][c] -= factor * a[col][c];
            }
            b[r] -= factor * b[col];
        }
    }
    let mut x = vec![0.0_f64; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for c in (r + 1)..n {
            s -= a[r][c] * x[c];
        }
        x[r] = s / a[r][r];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bearing, BearingKind, Load, LoadKind, Material, Member, Project};

    fn simply_supported() -> Project {
        let mut p = Project::default();
        p.members = vec![Member {
            length_mm: 4000.0,
            i_cm4: 4500.0,
            e_kn_cm2: 965.0,
            type_name: "Test".to_string(),
            perm_m_knm: 100.0,
            perm_v_kn: 100.0,
            parts: 1,
            material: Material::Timber,
            self_weight_kn_m: 0.0,
            gauge: crate::model::Gauge::Custom,
        }];
        p.bearings = vec![
            Bearing { node: 0, kind: BearingKind::Sliding, ..Default::default() },
            Bearing { node: 1, kind: BearingKind::Sliding, ..Default::default() },
        ];
        p.loads = vec![Load {
            kind: LoadKind::Trapezoid,
            active: true,
            a: 5.0,
            b: 5.0,
            c: 0.0,
            d: 4000.0,
            ..Default::default()
        }];
        p.calc.eurocode = false;
        p
    }

    #[test]
    fn simply_supported_reactions_and_moment() {
        let p = simply_supported();
        let r = solve(&p).expect("solve");
        assert!((r.sum_reactions_kn - 20.0).abs() < 1e-6, "sum={}", r.sum_reactions_kn);
        assert!((r.reactions[0].force_kn - 10.0).abs() < 1e-6);
        assert!((r.max_m - 10.0).abs() < 0.05, "max M={}", r.max_m);
        assert!(r.max_defl > 0.0);
    }

    #[test]
    fn cantilever_moment_is_hogging() {
        let mut p = simply_supported();
        p.bearings = vec![Bearing { node: 0, kind: BearingKind::Fixed, ..Default::default() }];
        p.loads = vec![Load {
            kind: LoadKind::Point,
            active: true,
            b: 10.0,
            c: 4000.0,
            ..Default::default()
        }];
        let r = solve(&p).expect("solve");
        assert!((r.sum_reactions_kn - 10.0).abs() < 1e-6);
        assert!((r.members[0].m_min + 40.0).abs() < 0.1, "Mmin={}", r.members[0].m_min);
    }
}
