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
