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
