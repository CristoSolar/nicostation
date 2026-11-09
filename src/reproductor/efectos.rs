//! Efectos visuales con shaders (GLSL ES 1.00: andan en WebGL1, o sea en la exportación web):
//! destello (flash), silueta (outline), escala de grises (grayscale) y ondas (wave), más la mezcla aditiva
//! de las partículas. Los materiales se crean una sola vez.

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams, ShaderSource, UniformDesc, UniformType};
use macroquad::prelude::*;

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec4 color;
varying highp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

/// Un solo shader con `mode`: 1 destello, 2 silueta, 3 grises, 4 ondas. Fuera de [0, 1] en uv es transparente
/// (para la silueta y las ondas el sprite se dibuja con un margen alrededor).
const EFFECT: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying highp vec2 uv;
uniform sampler2D Texture;
uniform float mode;
uniform float amount;
uniform float time;
uniform vec2 texel;
vec4 tex(vec2 p) {
    if (p.x < 0.0 || p.y < 0.0 || p.x > 1.0 || p.y > 1.0) { return vec4(0.0); }
    return texture2D(Texture, p);
}
void main() {
    vec2 p = uv;
    if (mode > 3.5) {
        p.x += sin(p.y * 12.0 + time * 6.0) * 0.04 * amount;
        p.y += sin(p.x * 9.0 + time * 4.0) * 0.02 * amount;
    }
    vec4 c = tex(p) * color;
    if (mode > 0.5 && mode < 1.5) {
        c.rgb = mix(c.rgb, vec3(1.0), clamp(amount, 0.0, 1.0));
    } else if (mode > 1.5 && mode < 2.5) {
        float n = tex(p + vec2(texel.x, 0.0)).a + tex(p - vec2(texel.x, 0.0)).a
                + tex(p + vec2(0.0, texel.y)).a + tex(p - vec2(0.0, texel.y)).a
                + tex(p + texel).a + tex(p - texel).a
                + tex(p + vec2(texel.x, -texel.y)).a + tex(p + vec2(-texel.x, texel.y)).a;
        float own = tex(p).a;
        if (own < 0.5 && n > 0.5) {
            c = vec4(color.rgb, color.a * clamp(amount, 0.0, 1.0));
        }
    } else if (mode > 2.5 && mode < 3.5) {
        float g = dot(c.rgb, vec3(0.299, 0.587, 0.114));
        c.rgb = mix(c.rgb, vec3(g), clamp(amount, 0.0, 1.0));
    }
    gl_FragColor = c;
}
"#;

const PLAIN: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying highp vec2 uv;
uniform sampler2D Texture;
void main() {
    gl_FragColor = texture2D(Texture, uv) * color;
}
"#;

/// Efecto de una instancia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    Flash,
    Outline,
    Grayscale,
    Wave,
}

impl Effect {
    /// Nombre de la propiedad «effect» (con algunos sinónimos en castellano). Con intensidad 0, ninguno.
    pub fn parse(name: &str, amount: f32) -> Option<Effect> {
        if amount <= 0. {
            return None;
        }
        Some(match name {
            "flash" | "destello" => Effect::Flash,
            "outline" | "silueta" | "borde" => Effect::Outline,
            "grayscale" | "greyscale" | "grises" => Effect::Grayscale,
            "wave" | "ondas" => Effect::Wave,
            _ => return None,
        })
    }

    fn mode(self) -> f32 {
        match self {
            Effect::Flash => 1.,
            Effect::Outline => 2.,
            Effect::Grayscale => 3.,
            Effect::Wave => 4.,
        }
    }

    /// Necesita dibujar un margen alrededor del sprite (lo que sale de su rectángulo).
    pub fn needs_margin(self) -> bool {
        matches!(self, Effect::Outline | Effect::Wave)
    }

    /// Lo mismo, pero sin shader: para rectángulos de color.
    pub fn apply_color(self, c: Color, amount: f32) -> Color {
        let a = amount.clamp(0., 1.);
        match self {
            Effect::Flash => Color::new(c.r + (1. - c.r) * a, c.g + (1. - c.g) * a, c.b + (1. - c.b) * a, c.a),
            Effect::Grayscale => {
                let g = c.r * 0.299 + c.g * 0.587 + c.b * 0.114;
                Color::new(c.r + (g - c.r) * a, c.g + (g - c.g) * a, c.b + (g - c.b) * a, c.a)
            }
            _ => c,
        }
    }
}

pub struct Materials {
    effect: Option<Material>,
    additive: Option<Material>,
}

fn alpha_blend() -> PipelineParams {
    PipelineParams {
        color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
        alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
        ..Default::default()
    }
}

impl Materials {
    /// Si la placa de video no compila un shader, el juego sigue sin ese efecto.
    pub fn new() -> Materials {
        let effect = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: EFFECT },
            MaterialParams {
                pipeline_params: alpha_blend(),
                uniforms: vec![
                    UniformDesc::new("mode", UniformType::Float1),
                    UniformDesc::new("amount", UniformType::Float1),
                    UniformDesc::new("time", UniformType::Float1),
                    UniformDesc::new("texel", UniformType::Float2),
                ],
                ..Default::default()
            },
        )
        .map_err(|e| eprintln!("efectos desactivados: {e:?}"))
        .ok();
        let additive = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: PLAIN },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One)),
                    alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One)),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(|e| eprintln!("mezcla aditiva desactivada: {e:?}"))
        .ok();
        Materials { effect, additive }
    }

    /// Activa el efecto para lo que se dibuje a continuación. `texel`: tamaño de un píxel de la silueta en uv.
    /// Devuelve `false` si no hay shader (se dibuja normal).
    pub fn use_effect(&self, e: Effect, amount: f32, time: f32, texel: Vec2) -> bool {
        let Some(m) = &self.effect else { return false };
        m.set_uniform("mode", e.mode());
        m.set_uniform("amount", amount);
        m.set_uniform("time", time);
        m.set_uniform("texel", texel);
        gl_use_material(m);
        true
    }

    pub fn use_additive(&self) -> bool {
        let Some(m) = &self.additive else { return false };
        gl_use_material(m);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_de_efectos() {
        assert_eq!(Effect::parse("flash", 1.), Some(Effect::Flash));
        assert_eq!(Effect::parse("silueta", 0.5), Some(Effect::Outline));
        assert_eq!(Effect::parse("wave", 0.), None, "intensidad 0 = sin efecto");
        assert_eq!(Effect::parse("none", 1.), None);
        assert_eq!(Effect::parse("", 1.), None);
        let c = Effect::Flash.apply_color(Color::new(0., 0.5, 1., 0.3), 0.5);
        assert_eq!((c.r, c.g, c.b, c.a), (0.5, 0.75, 1., 0.3));
        let g = Effect::Grayscale.apply_color(Color::new(1., 0., 0., 1.), 1.);
        assert!((g.r - 0.299).abs() < 1e-6 && (g.g - 0.299).abs() < 1e-6);
    }
}
