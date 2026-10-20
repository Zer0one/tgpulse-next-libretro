//! MAME model1_v.cpp direct display-list primitives (70e0c03dad, 9388840bcd).
use super::*;

pub(super) fn push_direct(system: &Model1System, view: &View, list: &[u8],
    mut offset: usize, queue: &mut Vec<Quad>, stats: &mut Model1RenderStats) -> usize {
    let limit = list.len() / 2;
    if offset + 20 > limit { return limit; }
    let mut texture = readi(list, offset + 2);
    // Direct coordinates are already screen-space, including z <= 0. Never
    // apply the perspective/frustum rules used by normal 3D objects.
    let point = |at: usize| {
        let x = readf(list, at);
        let y = readf(list, at + 2);
        Point { x, y, z: readf(list, at + 4),
            sx: (view.xc as f32 + x + view.ws_shift) as i32,
            sy: (view.yc as f32 - y) as i32 }
    };
    let mut old0 = point(offset + 6);
    let mut old1 = point(offset + 14);
    offset += 18;
    for _ in 0..MAX_LIST_COMMANDS {
        if offset + 4 > limit { return limit; }
        let flags = readi(list, offset + 2);
        let kind = flags & 3;
        if kind == 0 { return offset + 4; }
        let words = if kind == 2 { 12 } else { 20 };
        if offset + words + 2 > limit { return limit; }
        if flags & 0x1000 != 0 { texture = texture.wrapping_add(1); }
        let luminosity = ((readi(list, offset + 4) >> 24) / 2).min(63) as usize;
        let p0 = point(offset + 8);
        let p1 = if kind == 2 { p0 } else { point(offset + 16) };
        let depth = if kind == 2 { p0.z } else { readf(list, offset + 14) };
        let link = (flags >> 8) & 3;
        if link != 0 {
            let tex = texture.checked_sub(TGP_RAM_BASE)
                .and_then(|i| system.video.tgp_ram.get(i as usize)).copied().unwrap_or(0);
            let color = u16_at(&system.palette_ram, 0x1000 | (tex as usize & 0x3ff));
            let channel = |shift: u32, bank: usize| {
                let index = (((color >> shift) & 31u16) as usize) << 8;
                expand5(u32::from((u16_at(&system.colorxlat_ram, index | bank | luminosity) >> 3) & 31))
            };
            let moire = flags & 0x2000 != 0;
            let color = if moire && tex & 0x3ff == 0 {
                crate::tilemap::pen_color(system, 0)
            } else { 0xff00_0000 | (channel(0, 0) << 16) | (channel(5, 0x2000) << 8) | channel(10, 0x4000) };
            let points = [old1, old0, p0, p1];
            // Wing War's constant moire record is a point, not a visible dot.
            if !moire || !points.iter().all(|p| p.sx == points[0].sx && p.sy == points[0].sy) {
                queue.push(Quad { points, z: depth, color, moire });
                stats.source_quads += 1;
            }
        }
        match link {
            0 | 2 => { old0 = p0; old1 = p1; }
            1 => old1 = p0,
            _ => old0 = p1,
        }
        offset += words;
    }
    limit
}
