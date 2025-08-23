use bevy_ecs::prelude::*;

/// 2D point in XZ plane (y ignored) for navigation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct P2(pub f32, pub f32);

#[inline] fn dot(a: P2, b: P2) -> f32 { a.0*b.0 + a.1*b.1 }
#[inline] fn sub(a: P2, b: P2) -> P2 { P2(a.0-b.0, a.1-b.1) }
#[inline] fn len(a: P2) -> f32 { (a.0*a.0 + a.1*a.1).sqrt() }

/// Convex polygon (CCW) in XZ plane
#[derive(Debug, Clone)]
pub struct Poly { pub id: u32, pub verts: Vec<P2> }

/// Lightweight navmesh over convex polygons
#[derive(Resource, Debug, Default, Clone)]
pub struct NavMesh {
    pub polys: Vec<Poly>,
    /// adjacency graph: poly index -> neighbor poly indices
    pub adj: Vec<Vec<usize>>, 
}

impl NavMesh {
    pub fn from_rects(rects: &[(f32,f32,f32,f32)]) -> Self {
        // rect: (minx, minz, maxx, maxz)
        let mut polys = Vec::new();
        for (i, r) in rects.iter().enumerate() {
            polys.push(Poly { id: i as u32, verts: vec![P2(r.0,r.1), P2(r.2,r.1), P2(r.2,r.3), P2(r.0,r.3)] });
        }
        let mut adj = vec![Vec::new(); polys.len()];
        for i in 0..polys.len() { for j in (i+1)..polys.len() { if shares_edge(&polys[i], &polys[j]) { adj[i].push(j); adj[j].push(i); } } }
        NavMesh { polys, adj }
    }

    pub fn nearest_poly(&self, p: P2) -> Option<usize> {
        let mut best: Option<(usize,f32)> = None;
        for (i, poly) in self.polys.iter().enumerate() {
            let d = distance_to_poly(poly, p);
            if best.map_or(true, |(_,bd)| d<bd) { best = Some((i,d)); }
        }
        best.map(|(i,_)| i)
    }

    /// Return world-space XZ point clamped to polygon interior/boundary (project-to-poly)
    pub fn clamp_to_poly(&self, idx: usize, p: P2) -> P2 { project_to_poly(&self.polys[idx], p) }

    /// Distance to the closest boundary edge for proximity cues.
    pub fn boundary_distance(&self, idx: usize, p: P2) -> f32 { distance_to_poly_edge(&self.polys[idx], p) }

    /// A* across polygon centers; returns waypoints as XZ centers
    pub fn astar(&self, start_idx: usize, goal_idx: usize) -> Option<Vec<P2>> {
        use std::collections::{BinaryHeap, HashMap};
        #[derive(Copy, Clone, Eq, PartialEq)]
        struct Node { f: u32, i: usize }
        impl Ord for Node { fn cmp(&self, o: &Self) -> std::cmp::Ordering { o.f.cmp(&self.f) } }
        impl PartialOrd for Node { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) } }
        let center = |i: usize| poly_center(&self.polys[i]);
        let h = |a: usize, b: usize| -> u32 { let d = sub(center(a), center(b)); (len(d)) as u32 };
        let mut open = BinaryHeap::new();
        let mut g: HashMap<usize,u32> = HashMap::new();
        let mut came: HashMap<usize,usize> = HashMap::new();
        g.insert(start_idx, 0);
        open.push(Node { f: h(start_idx, goal_idx), i: start_idx });
        while let Some(Node { i, .. }) = open.pop() {
            if i == goal_idx { break; }
            let gi = *g.get(&i).unwrap_or(&u32::MAX);
            for &nb in &self.adj[i] {
                let tentative = gi.saturating_add(1);
                if tentative < *g.get(&nb).unwrap_or(&u32::MAX) {
                    g.insert(nb, tentative);
                    came.insert(nb, i);
                    open.push(Node { f: tentative.saturating_add(h(nb, goal_idx)), i: nb });
                }
            }
        }
        if !came.contains_key(&goal_idx) && start_idx != goal_idx { return None; }
        let mut order = vec![goal_idx];
        let mut cur = goal_idx;
        while let Some(&prev) = came.get(&cur) { cur = prev; order.push(cur); if cur == start_idx { break; } }
        order.reverse();
        Some(order.into_iter().map(center).collect())
    }
}

fn poly_center(p: &Poly) -> P2 {
    let mut c = P2(0.0,0.0);
    if p.verts.is_empty() { return c; }
    for v in &p.verts { c.0 += v.0; c.1 += v.1; }
    c.0 /= p.verts.len() as f32; c.1 /= p.verts.len() as f32; c
}

fn shares_edge(a: &Poly, b: &Poly) -> bool {
    // consider two edges sharing if they have two vertices approximately equal
    let eps = 1e-4;
    for i in 0..a.verts.len() {
        let a0 = a.verts[i]; let a1 = a.verts[(i+1)%a.verts.len()];
        for j in 0..b.verts.len() {
            let b0 = b.verts[j]; let b1 = b.verts[(j+1)%b.verts.len()];
            let m1 = (a0.0-b1.0).abs() < eps && (a0.1-b1.1).abs() < eps && (a1.0-b0.0).abs() < eps && (a1.1-b0.1).abs() < eps;
            let m2 = (a0.0-b0.0).abs() < eps && (a0.1-b0.1).abs() < eps && (a1.0-b1.0).abs() < eps && (a1.1-b1.1).abs() < eps;
            if m1 || m2 { return true; }
        }
    }
    false
}

fn distance_to_poly(poly: &Poly, p: P2) -> f32 {
    if point_in_poly(poly, p) { 0.0 } else { distance_to_poly_edge(poly, p) }
}

fn distance_to_poly_edge(poly: &Poly, p: P2) -> f32 {
    let mut best = f32::INFINITY;
    for i in 0..poly.verts.len() {
        let a = poly.verts[i];
        let b = poly.verts[(i+1)%poly.verts.len()];
        best = best.min(point_segment_distance(p, a, b));
    }
    best
}

fn point_segment_distance(p: P2, a: P2, b: P2) -> f32 {
    let ab = sub(b,a); let ap = sub(p,a);
    let t = (dot(ap, ab) / (dot(ab,ab).max(1e-12))).clamp(0.0, 1.0);
    len(sub(P2(a.0 + ab.0*t, a.1 + ab.1*t), p))
}

fn point_in_poly(poly: &Poly, p: P2) -> bool {
    // winding method for convex is fine; for robustness use half-space check
    let n = poly.verts.len();
    if n < 3 { return false; }
    for i in 0..n {
        let a = poly.verts[i]; let b = poly.verts[(i+1)%n];
        let edge = sub(b,a); let to_p = sub(p,a);
        // left of edge (CCW) => inside
        if edge.0*to_p.1 - edge.1*to_p.0 < -1e-5 { return false; }
    }
    true
}

fn project_to_poly(poly: &Poly, p: P2) -> P2 {
    if point_in_poly(poly, p) { return p; }
    // clamp to nearest edge projection
    let mut best = (P2(0.0,0.0), f32::INFINITY);
    for i in 0..poly.verts.len() {
        let a = poly.verts[i];
        let b = poly.verts[(i+1)%poly.verts.len()];
        let ab = sub(b,a); let ap = sub(p,a);
        let t = (dot(ap, ab) / (dot(ab,ab).max(1e-12))).clamp(0.0, 1.0);
        let q = P2(a.0 + ab.0*t, a.1 + ab.1*t);
        let d = len(sub(q,p));
        if d < best.1 { best = (q,d); }
    }
    best.0
}

pub fn vec3_to_p2(v: glam::Vec3) -> P2 { P2(v.x, v.z) }
pub fn p2_to_vec3(p: P2, y: f32) -> glam::Vec3 { glam::vec3(p.0, y, p.1) }
