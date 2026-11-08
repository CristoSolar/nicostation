//! Sonido con kira: efectos y música (sin posición todavía; el audio 3D espacial viene con `PlaySound3d`).
//! Si no hay tarjeta de sonido (o en `--captura`) todo sigue andando en silencio.

use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween};
use std::collections::HashMap;

pub struct Audio {
    manager: Option<AudioManager<DefaultBackend>>,
    datos: HashMap<String, StaticSoundData>,
    /// Sonidos sonando, por archivo (para «detener sonido»).
    sonando: Vec<(String, StaticSoundHandle)>,
    musica: Option<StaticSoundHandle>,
}

/// Volumen 0..100 (como en las hojas de eventos) a decibeles.
pub fn decibeles(volumen: f32) -> f32 {
    let v = (volumen / 100.).clamp(0., 4.);
    if v <= 0.001 { Decibels::SILENCE.0 } else { (20. * v.log10()).max(Decibels::SILENCE.0) }
}

impl Audio {
    pub fn new(activo: bool) -> Audio {
        let manager = if activo {
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
                .map_err(|e| eprintln!("sin sonido: {e}"))
                .ok()
        } else {
            None
        };
        Audio { manager, datos: HashMap::new(), sonando: vec![], musica: None }
    }

    /// Decodifica un sonido (para tenerlo listo antes de empezar).
    pub fn cargar(&mut self, archivo: &str, bytes: Vec<u8>) {
        if self.manager.is_none() || self.datos.contains_key(archivo) {
            return;
        }
        match StaticSoundData::from_cursor(std::io::Cursor::new(bytes)) {
            Ok(d) => {
                self.datos.insert(archivo.into(), d);
            }
            Err(e) => eprintln!("sonido {archivo}: {e}"),
        }
    }

    pub fn tiene(&self, archivo: &str) -> bool {
        self.manager.is_none() || self.datos.contains_key(archivo)
    }

    fn tocar(&mut self, archivo: &str, volumen: f32, bucle: bool) -> Option<StaticSoundHandle> {
        let m = self.manager.as_mut()?;
        let mut d = self.datos.get(archivo)?.volume(Decibels(decibeles(volumen)));
        if bucle {
            d = d.loop_region(..);
        }
        m.play(d).map_err(|e| eprintln!("sonido {archivo}: {e}")).ok()
    }

    pub fn sonido(&mut self, archivo: &str, volumen: f32) {
        self.sonando.retain(|(_, h)| h.state() != kira::sound::PlaybackState::Stopped);
        if let Some(h) = self.tocar(archivo, volumen, false) {
            self.sonando.push((archivo.into(), h));
        }
    }

    pub fn musica(&mut self, archivo: &str, volumen: f32) {
        self.parar_musica();
        self.musica = self.tocar(archivo, volumen, true);
    }

    pub fn parar_musica(&mut self) {
        if let Some(mut h) = self.musica.take() {
            h.stop(Tween::default());
        }
    }

    pub fn parar(&mut self, archivo: &str) {
        for (f, h) in &mut self.sonando {
            if f == archivo {
                h.stop(Tween::default());
            }
        }
        self.sonando.retain(|(f, _)| f != archivo);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn volumen_a_decibeles() {
        assert_eq!(super::decibeles(100.), 0.);
        assert!((super::decibeles(50.) + 6.02).abs() < 0.01);
        assert_eq!(super::decibeles(0.), -60.);
        assert_eq!(super::decibeles(-5.), -60.);
    }
}
