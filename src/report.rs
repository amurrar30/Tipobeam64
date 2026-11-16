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
