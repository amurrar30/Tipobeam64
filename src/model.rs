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
