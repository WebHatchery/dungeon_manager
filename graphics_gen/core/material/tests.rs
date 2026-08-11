use super::*;

fn luma(px: Rgba<u8>) -> f32 {
    0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32
}

/// The light vector must be normalized, or every diffuse term is silently
/// scaled and `ambient + diffuse` no longer means what the materials say.
#[test]
fn light_and_half_vectors_are_unit_length() {
    for (name, v) in [
        ("light", (LIGHT_X, LIGHT_Y, LIGHT_Z)),
        ("half", (HALF_X, HALF_Y, HALF_Z)),
    ] {
        let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
        assert!(
            (len - 1.0).abs() < 0.01,
            "{name} vector length {len} is not ~1.0"
        );
    }
}

/// `HALF_*` is `normalize(L + V)` precomputed. It is therefore a second
/// copy of the light direction, and the copy nothing recomputes is the one
/// that goes stale — move the key light and the specular highlight silently
/// keeps pointing at the old one. Derive it here and compare.
#[test]
fn half_vector_matches_the_light_it_was_derived_from() {
    // View direction: the camera's ray is `wy = py + wz * TILT`, so moving
    // toward the viewer is `(0, TILT, 1)`.
    let tilt = crate::graphics_gen::core::TILT;
    let v_len = (tilt * tilt + 1.0).sqrt();
    let view = (0.0, tilt / v_len, 1.0 / v_len);

    let sum = (LIGHT_X + view.0, LIGHT_Y + view.1, LIGHT_Z + view.2);
    let len = (sum.0 * sum.0 + sum.1 * sum.1 + sum.2 * sum.2).sqrt();
    let expected = (sum.0 / len, sum.1 / len, sum.2 / len);

    for (axis, got, want) in [
        ("x", HALF_X, expected.0),
        ("y", HALF_Y, expected.1),
        ("z", HALF_Z, expected.2),
    ] {
        assert!(
            (got - want).abs() < 0.005,
            "HALF_{} is {got}, but normalize(LIGHT + VIEW) gives {want} — \
             recompute the half-vector after moving the light",
            axis.to_uppercase()
        );
    }
}

/// The bug this test exists for: the diffuse dot product was negated, so
/// `nz = 1` (an upward-facing surface, which is most of a large rounded
/// body in this projection) received *zero* diffuse light. Big creatures
/// rendered as near-black silhouettes while small spheres — mostly rim —
/// looked fine.
#[test]
fn upward_facing_surfaces_are_lit_not_black() {
    let mat = Material::flesh(120, 140, 110);
    let top = shade_color((0.0, 0.0, 1.0), &mat, 0.0);
    let ambient_only = mat.base_color[1] as f32 * mat.ambient;

    assert!(
        luma(top) > ambient_only * 1.5,
        "top surface {top:?} is at ambient level — diffuse is not reaching it"
    );
}

/// Shading has to have a direction: the side facing the key light is
/// brighter than the side facing away, and the underside is darkest.
#[test]
fn shading_falls_off_away_from_the_key_light() {
    let mat = Material::matte(180, 180, 180);

    let toward = luma(shade_color((LIGHT_X, LIGHT_Y, LIGHT_Z), &mat, 0.0));
    let top = luma(shade_color((0.0, 0.0, 1.0), &mat, 0.0));
    let away = luma(shade_color((-LIGHT_X, -LIGHT_Y, -LIGHT_Z), &mat, 0.0));
    let under = luma(shade_color((0.0, 0.0, -1.0), &mat, 0.0));

    assert!(toward > top, "surface facing the light should be brightest");
    assert!(top > away, "top should out-light the shadow side");
    assert!(
        (away - under).abs() < 1.0,
        "both fully-shadowed normals should sit at ambient"
    );
    assert!(under > 0.0, "ambient should keep shadows off pure black");
}
