//! Game-specific sprite variation configs
//!
//! The variation engine (per-seed recoloring + texture cache) lives in
//! `macroquad_toolkit::sprite`; this module only registers Deep Dominion's
//! per-unit color regions into a toolkit cache.

use macroquad_toolkit::sprite::{ColorRegion, SpriteVariationCache, SpriteVariationConfig};

/// Build a variation cache pre-registered with configs for all unit types.
/// Unregistered sprite ids fall back to the toolkit's default two-region
/// config (matches the old local "default" config).
pub fn build_variation_cache() -> SpriteVariationCache {
    let mut cache = SpriteVariationCache::new();

    // Wizard/Warlock - robe and staff colors can vary
    cache.register(
        "wizard",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("robe", 220.0, 280.0, 0.3, 1.0, 1.5), // Blue/purple robes
                ColorRegion::new("staff_gem", 0.0, 360.0, 0.5, 1.0, 2.0), // Staff gem - high variation
                ColorRegion::new("trim", 30.0, 60.0, 0.3, 1.0, 1.0),      // Gold trim
            ],
            variation_strength: 0.8,
        },
    );

    cache.register(
        "warlock",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("robe", 260.0, 320.0, 0.2, 1.0, 1.2), // Purple/dark robes
                ColorRegion::new("magic_glow", 0.0, 360.0, 0.4, 1.0, 2.0), // Magic effects
                ColorRegion::new("skin", 20.0, 50.0, 0.2, 0.6, 0.5),   // Skin tone
            ],
            variation_strength: 0.7,
        },
    );

    // Knight - armor tint and plume color
    cache.register(
        "knight",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("armor", 0.0, 360.0, 0.0, 0.3, 0.3), // Metal armor (low sat)
                ColorRegion::new("plume", 0.0, 360.0, 0.5, 1.0, 2.0), // Helmet plume
                ColorRegion::new("cloth", 0.0, 60.0, 0.4, 1.0, 1.0),  // Cloth/cape
            ],
            variation_strength: 0.6,
        },
    );

    // Goblin - skin tone and equipment
    cache.register(
        "goblin",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("skin", 60.0, 150.0, 0.3, 0.8, 0.8), // Green skin
                ColorRegion::new("cloth", 0.0, 360.0, 0.3, 1.0, 1.5), // Clothing
                ColorRegion::new("eyes", 0.0, 60.0, 0.5, 1.0, 1.0),   // Eye color
            ],
            variation_strength: 0.7,
        },
    );

    // Orc - skin and war paint
    cache.register(
        "orc",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("skin", 60.0, 140.0, 0.3, 0.7, 0.6), // Green/brown skin
                ColorRegion::new("armor", 0.0, 60.0, 0.2, 0.8, 0.8),  // Leather/metal
                ColorRegion::new("warpaint", 0.0, 360.0, 0.5, 1.0, 1.5), // War paint
            ],
            variation_strength: 0.6,
        },
    );

    // Skeleton - bone color and equipment
    cache.register(
        "skeleton",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("bone", 30.0, 60.0, 0.1, 0.4, 0.4), // Bone color
                ColorRegion::new("glow", 180.0, 240.0, 0.3, 1.0, 1.2), // Soul glow
            ],
            variation_strength: 0.5,
        },
    );

    // Imp - skin and glow (dark maroon sprite with low saturation)
    cache.register(
        "imp",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("skin", 340.0, 40.0, 0.15, 1.0, 1.2), // Dark red/maroon skin (wraps around)
                ColorRegion::new("wings", 340.0, 60.0, 0.1, 0.8, 0.8), // Wing membrane
                ColorRegion::new("horns", 0.0, 60.0, 0.05, 0.4, 0.6),  // Dark horns/features
            ],
            variation_strength: 0.8,
        },
    );

    // Troll - skin and moss
    cache.register(
        "troll",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("skin", 80.0, 160.0, 0.2, 0.6, 0.7), // Gray-green skin
                ColorRegion::new("moss", 80.0, 140.0, 0.4, 0.9, 0.8), // Moss/growth
            ],
            variation_strength: 0.5,
        },
    );

    // Archer
    cache.register(
        "archer",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("cloth", 60.0, 150.0, 0.3, 0.8, 1.0), // Green clothing
                ColorRegion::new("leather", 20.0, 50.0, 0.3, 0.7, 0.5), // Brown leather
                ColorRegion::new("feather", 0.0, 360.0, 0.4, 1.0, 1.5), // Arrow fletching
            ],
            variation_strength: 0.6,
        },
    );

    // Vampire
    cache.register(
        "vampire",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("cape", 0.0, 360.0, 0.3, 1.0, 1.2), // Cape color
                ColorRegion::new("skin", 0.0, 40.0, 0.0, 0.3, 0.3),  // Pale skin
                ColorRegion::new("eyes", 0.0, 60.0, 0.6, 1.0, 1.5),  // Eye glow
            ],
            variation_strength: 0.7,
        },
    );

    // Spider
    cache.register(
        "spider",
        SpriteVariationConfig {
            color_regions: vec![
                ColorRegion::new("body", 0.0, 360.0, 0.1, 0.5, 0.6), // Body color
                ColorRegion::new("markings", 0.0, 60.0, 0.5, 1.0, 1.2), // Pattern markings
            ],
            variation_strength: 0.5,
        },
    );

    cache
}

#[cfg(test)]
mod tests;
