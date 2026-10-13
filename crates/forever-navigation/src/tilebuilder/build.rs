// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded composition of the separately attributed stages.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub heap_bytes: usize,
    pub work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            heap_bytes: 512 * 1024 * 1024,
            work: 16_000_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Statistics {
    pub subtiles: u16,
    pub nonempty_subtiles: u16,
    pub discarded_detail_faces: u32,
    pub work: u64,
    /// Conservative maximum reservation across stages, not measured RSS.
    pub heap_reservation: usize,
}
pub struct Data {
    payload: detour::Data,
    statistics: Statistics,
}
impl Data {
    pub fn payload(&self) -> &detour::Data {
        &self.payload
    }
    /// Consumes pristine construction output and applies the original offline
    /// producer's fresh-navmesh save mutation. Whole-world loading is separate.
    pub fn into_saved(
        self,
        params: mmap::Params,
        limits: detour::SaveLimits,
    ) -> Result<detour::SavedData, detour::SaveError> {
        self.payload.prepare_for_save(params, limits)
    }
    pub const fn statistics(&self) -> Statistics {
        self.statistics
    }
}
pub enum Outcome {
    Empty(Statistics),
    Tile(Data),
}
fn remaining(limits: Limits, resident: usize, work: u64) -> Result<(usize, u64), Error> {
    Ok((
        limits
            .heap_bytes
            .checked_sub(resident)
            .ok_or(Error::Limit)?,
        limits.work.checked_sub(work).ok_or(Error::Limit)?,
    ))
}
/// Processes all 625 subtiles, in original order, with complete supplied mesh and
/// already selected connections. A failing subtile aborts the whole operation;
/// original continue-on-failure partial tiles are never returned as success.
/// Caller mesh/connections are borrowed and outside the owned-capacity budget.
pub fn build(
    mesh: &Mesh,
    connections: &[offmesh::Connection],
    request: Request,
    params: mmap::Params,
    limits: Limits,
) -> Result<Outcome, Error> {
    let max = Limits::default();
    if limits.heap_bytes > max.heap_bytes || limits.work > max.work {
        return Err(Error::Limit);
    }
    let cfg = Config::new(request, mesh)?;
    let options = cfg.detour_options(params)?;
    let mut pieces = Vec::new();
    let count = usize::from(SUBTILES) * usize::from(SUBTILES);
    if count * std::mem::size_of::<recast::TileMesh>() > limits.heap_bytes {
        return Err(Error::Limit);
    }
    pieces
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    let mut resident = pieces.capacity() * std::mem::size_of::<recast::TileMesh>();
    let mut stats = Statistics::default();
    for y in 0..SUBTILES {
        for x in 0..SUBTILES {
            let index = x + y * SUBTILES;
            let (heap, work) = remaining(limits, resident, stats.work)?;
            let mut stage = recast::Limits::default();
            stage.heap_bytes = stage.heap_bytes.min(heap);
            stage.work = stage.work.min(work);
            stats.heap_reservation = stats.heap_reservation.max(resident + stage.heap_bytes);
            let run = || -> Result<recast::TileMesh, recast::Error> {
                Ok(recast::build(
                    mesh,
                    cfg.grid(x, y).expect("bounded loop"),
                    cfg.request.slopes,
                    cfg.agent,
                    stage,
                )?
                .prepare(cfg.radius)?
                .build_regions(cfg.region_options())?
                .build_contours(cfg.contour_options())?
                .build_mesh(6)?
                .build_detail(cfg.detail_options())?
                .into_tile_mesh())
            };
            let piece = run().map_err(|source| Error::Subtile { index, source })?;
            resident = resident
                .checked_add(piece.capacity_bytes())
                .ok_or(Error::Limit)?;
            stats.work = stats
                .work
                .checked_add(piece.processing_work())
                .ok_or(Error::Limit)?;
            remaining(limits, resident, stats.work)?;
            stats.subtiles += 1;
            stats.nonempty_subtiles += u16::from(!piece.polygons().is_empty());
            pieces.push(piece);
        }
    }
    // MergeLimits itself charges the consumed pieces and vector, so do not
    // subtract resident a second time from that stage's total allowance.
    let mut merge = recast::MergeLimits::default();
    merge.heap_bytes = merge.heap_bytes.min(limits.heap_bytes);
    merge.work = merge.work.min(limits.work - stats.work);
    stats.heap_reservation = stats.heap_reservation.max(merge.heap_bytes);
    let merged = recast::TileMesh::merge(pieces, merge).map_err(Error::Merge)?;
    stats.work = merged.processing_work();
    stats.discarded_detail_faces = merged.discarded_faces();
    if merged.vertices().is_empty() || merged.polygons().is_empty() {
        return Ok(Outcome::Empty(stats));
    }
    let (heap, work) = remaining(limits, merged.capacity_bytes(), stats.work)?;
    let mut d = detour::Limits::default();
    d.heap_bytes = d.heap_bytes.min(heap);
    d.work = d.work.min(work);
    stats.heap_reservation = stats
        .heap_reservation
        .max(merged.capacity_bytes() + d.heap_bytes);
    let payload = detour::build(&merged, connections, options, d).map_err(Error::Detour)?;
    stats.work = stats
        .work
        .checked_add(payload.statistics().work)
        .ok_or(Error::Limit)?;
    if payload.statistics().detail_base_wraps != 0
        || payload.statistics().neighbor_marker_collisions != 0
    {
        return Err(Error::PayloadDiagnostics);
    }
    Ok(Outcome::Tile(Data {
        payload,
        statistics: stats,
    }))
}
