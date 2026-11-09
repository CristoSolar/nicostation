//! Sonido del reproductor 2D: canales, volumen por sonido, fundidos de música y de canal.
//!
//! La lógica (qué suena, a qué volumen, qué se apaga) está en [`Mixer`], que no sabe nada del sistema de
//! sonido: le habla a un [`Backend`]. En escritorio el backend es kira; en web, el plugin de audio que ya
//! trae `mq_js_bundle.js` (WebAudio). Los fundidos se hacen rampeando el volumen en cada cuadro.

use nicostation::runtime::Cmd;
use std::collections::HashMap;

/// Canal de la música: «Volumen del canal musica» cambia la música.
pub const MUSIC: &str = "musica";

/// Lo mínimo que necesita el mezclador. Los volúmenes van de 0 a 1 (o más, amplificando).
pub trait Backend {
    /// ¿Ya está listo el archivo para sonar?
    fn has(&self, file: &str) -> bool;
    /// Empieza a sonar; devuelve un identificador de esta reproducción.
    fn play(&mut self, file: &str, volume: f32, looped: bool) -> Option<u64>;
    fn set_volume(&mut self, id: u64, volume: f32);
    fn stop(&mut self, id: u64);
    /// ¿Sigue sonando? (para olvidar los sonidos cortos que terminaron)
    fn playing(&self, id: u64) -> bool;
}

/// Un valor que va de `from` a `to` en `dur` segundos (lineal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ramp {
    pub from: f32,
    pub to: f32,
    pub t: f32,
    pub dur: f32,
}

impl Ramp {
    pub fn fixed(v: f32) -> Ramp {
        Ramp { from: v, to: v, t: 0., dur: 0. }
    }
    pub fn new(from: f32, to: f32, dur: f32) -> Ramp {
        Ramp { from, to, t: 0., dur: dur.max(0.) }
    }
    pub fn value(&self) -> f32 {
        if self.dur <= 0. || self.t >= self.dur {
            self.to
        } else {
            self.from + (self.to - self.from) * (self.t / self.dur)
        }
    }
    pub fn done(&self) -> bool {
        self.dur <= 0. || self.t >= self.dur
    }
    pub fn advance(&mut self, dt: f32) {
        self.t = (self.t + dt).min(self.dur.max(0.));
    }
    /// Seguir desde donde está ahora hacia `to`.
    pub fn retarget(&mut self, to: f32, dur: f32) {
        *self = Ramp::new(self.value(), to, dur);
    }
}

#[derive(Debug)]
struct Play {
    id: u64,
    file: String,
    /// Canal ("" = sin canal). La música va en [`MUSIC`].
    channel: String,
    music: bool,
    /// Volumen propio del sonido (0..1).
    volume: f32,
    /// Fundido de entrada/salida (0..1).
    fade: Ramp,
    /// Se apaga cuando el fundido termina.
    dying: bool,
    /// Último volumen enviado al backend.
    sent: f32,
}

#[derive(Default)]
pub struct Mixer {
    plays: Vec<Play>,
    channels: HashMap<String, Ramp>,
}

/// Volumen de las hojas de eventos (0..100) a 0..1; se permite hasta el doble.
fn vol(v: f32) -> f32 {
    (v / 100.).clamp(0., 2.)
}

impl Mixer {
    fn channel_volume(&self, ch: &str) -> f32 {
        self.channels.get(ch).map_or(1., Ramp::value)
    }

    fn effective(&self, p: &Play) -> f32 {
        let ch = if p.music { MUSIC } else { p.channel.as_str() };
        p.volume * p.fade.value() * if ch.is_empty() { 1. } else { self.channel_volume(ch) }
    }

    fn start(&mut self, b: &mut impl Backend, file: &str, volume: f32, channel: &str, music: bool, fade: f32) {
        let mut p = Play { id: 0, file: file.into(), channel: channel.into(), music, volume, fade: Ramp::new(if fade > 0. { 0. } else { 1. }, 1., fade), dying: false, sent: 0. };
        let v = self.effective(&p);
        if let Some(id) = b.play(file, v, music) {
            p.id = id;
            p.sent = v;
            self.plays.push(p);
        }
    }

    fn stop_where(&mut self, b: &mut impl Backend, f: impl Fn(&Play) -> bool) {
        self.plays.retain(|p| {
            let hit = f(p);
            if hit {
                b.stop(p.id);
            }
            !hit
        });
    }

    /// Atiende una orden de sonido del motor. Devuelve `false` si no era de sonido.
    pub fn command(&mut self, b: &mut impl Backend, cmd: &Cmd) -> bool {
        match cmd {
            Cmd::PlaySound { file, volume, channel } => {
                if !channel.is_empty() {
                    self.stop_where(b, |p| !p.music && p.channel == *channel);
                }
                self.start(b, file, vol(*volume), channel, false, 0.);
            }
            Cmd::PlayMusic { file, volume, fade } => {
                let fade = fade.max(0.);
                // La misma música que ya suena: solo cambia el volumen (no vuelve a empezar).
                if let Some(p) = self.plays.iter_mut().find(|p| p.music && !p.dying && p.file == *file) {
                    p.volume = vol(*volume);
                    p.fade.retarget(1., fade);
                    for o in self.plays.iter_mut().filter(|o| o.music && o.file != *file && !o.dying) {
                        o.dying = true;
                        o.fade.retarget(0., fade);
                    }
                    return true;
                }
                if fade > 0. {
                    for p in self.plays.iter_mut().filter(|p| p.music) {
                        p.dying = true;
                        p.fade.retarget(0., fade);
                    }
                } else {
                    self.stop_where(b, |p| p.music);
                }
                self.start(b, file, vol(*volume), MUSIC, true, fade);
            }
            Cmd::ChannelVolume { channel, volume, fade } => {
                self.channels.entry(channel.clone()).or_insert(Ramp::fixed(1.)).retarget(vol(*volume), fade.max(0.));
            }
            Cmd::StopChannel { channel } => {
                let music = channel == MUSIC;
                self.stop_where(b, |p| if music { p.music } else { !p.music && p.channel == *channel });
            }
            Cmd::StopMusic => self.stop_where(b, |p| p.music),
            Cmd::StopSound { file } => self.stop_where(b, |p| !p.music && p.file == *file),
            _ => return false,
        }
        true
    }

    /// Avanza los fundidos `dt` segundos y manda los volúmenes nuevos.
    pub fn update(&mut self, b: &mut impl Backend, dt: f32) {
        for r in self.channels.values_mut() {
            r.advance(dt);
        }
        for i in 0..self.plays.len() {
            self.plays[i].fade.advance(dt);
            let v = self.effective(&self.plays[i]);
            let p = &mut self.plays[i];
            if (v - p.sent).abs() > 1e-4 {
                b.set_volume(p.id, v);
                p.sent = v;
            }
        }
        self.stop_where(b, |p| p.dying && p.fade.done());
        self.plays.retain(|p| b.playing(p.id));
    }

    /// Cuántas reproducciones sigue el mezclador (para pruebas y depuración).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn len(&self) -> usize {
        self.plays.len()
    }
}

// ---------- Escritorio: kira ----------

#[cfg(not(target_arch = "wasm32"))]
pub use native::Native as Platform;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
    use kira::sound::PlaybackState;
    use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween};
    use std::collections::HashMap;

    /// Sin tarjeta de sonido (o en `--captura`) todo sigue andando, en silencio.
    pub struct Native {
        manager: Option<AudioManager<DefaultBackend>>,
        data: HashMap<String, StaticSoundData>,
        handles: HashMap<u64, StaticSoundHandle>,
        next: u64,
    }

    fn db(v: f32) -> Decibels {
        if v <= 0.001 { Decibels::SILENCE } else { Decibels((20. * v.log10()).max(Decibels::SILENCE.0)) }
    }

    /// Cambios de volumen de un cuadro: un poco suaves para que no haya chasquidos.
    fn smooth() -> Tween {
        Tween { duration: std::time::Duration::from_millis(20), ..Default::default() }
    }

    impl Native {
        pub fn new(active: bool) -> Native {
            let manager = if active {
                AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).map_err(|e| eprintln!("sin sonido: {e}")).ok()
            } else {
                None
            };
            Native { manager, data: HashMap::new(), handles: HashMap::new(), next: 1 }
        }

        /// Decodifica un sonido para tenerlo listo.
        pub fn load(&mut self, file: &str, bytes: Vec<u8>) {
            if self.manager.is_none() || self.data.contains_key(file) {
                return;
            }
            match StaticSoundData::from_cursor(std::io::Cursor::new(bytes)) {
                Ok(d) => {
                    self.data.insert(file.into(), d);
                }
                Err(e) => eprintln!("sonido {file}: {e}"),
            }
        }

        /// En escritorio la decodificación es inmediata.
        pub fn loaded(&self, _file: &str) -> bool {
            true
        }
    }

    impl super::Backend for Native {
        fn has(&self, file: &str) -> bool {
            self.manager.is_none() || self.data.contains_key(file)
        }
        fn play(&mut self, file: &str, volume: f32, looped: bool) -> Option<u64> {
            let m = self.manager.as_mut()?;
            let mut d = self.data.get(file)?.volume(db(volume));
            if looped {
                d = d.loop_region(..);
            }
            let h = m.play(d).map_err(|e| eprintln!("sonido {file}: {e}")).ok()?;
            let id = self.next;
            self.next += 1;
            self.handles.insert(id, h);
            Some(id)
        }
        fn set_volume(&mut self, id: u64, volume: f32) {
            if let Some(h) = self.handles.get_mut(&id) {
                h.set_volume(db(volume), smooth());
            }
        }
        fn stop(&mut self, id: u64) {
            if let Some(mut h) = self.handles.remove(&id) {
                h.stop(smooth());
            }
        }
        fn playing(&self, id: u64) -> bool {
            self.handles.get(&id).is_some_and(|h| h.state() != PlaybackState::Stopped)
        }
    }
}

// ---------- Web: plugin «macroquad_audio» de mq_js_bundle.js ----------

#[cfg(target_arch = "wasm32")]
pub use web::Web as Platform;

#[cfg(target_arch = "wasm32")]
mod web {
    use std::collections::HashMap;

    // Funciones del plugin de audio que trae mq_js_bundle.js (las mismas que usa quad-snd).
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn audio_init();
        fn audio_add_buffer(content: *const u8, content_len: u32) -> u32;
        fn audio_source_is_loaded(sound_key: u32) -> bool;
        fn audio_play_buffer(sound_key: u32, volume: f32, repeat: bool) -> u32;
        fn audio_playback_stop(playback_key: u32);
        fn audio_playback_set_volume(playback_key: u32, volume: f32);
    }

    pub struct Web {
        keys: HashMap<String, u32>,
        /// Reproducción → (archivo, cuándo termina en segundos de reloj; infinito si se repite).
        plays: HashMap<u64, f64>,
    }

    impl Web {
        pub fn new(_active: bool) -> Web {
            unsafe { audio_init() };
            Web { keys: HashMap::new(), plays: HashMap::new() }
        }
        pub fn load(&mut self, file: &str, bytes: Vec<u8>) {
            if !self.keys.contains_key(file) {
                let k = unsafe { audio_add_buffer(bytes.as_ptr(), bytes.len() as u32) };
                self.keys.insert(file.into(), k);
            }
        }
        /// La decodificación en el navegador es asíncrona.
        pub fn loaded(&self, file: &str) -> bool {
            self.keys.get(file).is_some_and(|&k| unsafe { audio_source_is_loaded(k) })
        }
    }

    impl super::Backend for Web {
        fn has(&self, file: &str) -> bool {
            self.keys.contains_key(file)
        }
        fn play(&mut self, file: &str, volume: f32, looped: bool) -> Option<u64> {
            let k = *self.keys.get(file)?;
            let id = unsafe { audio_play_buffer(k, volume, looped) } as u64;
            // El plugin no avisa cuándo termina un sonido: se olvida a los 30 s (detenerlo después no hace nada).
            let end = if looped { f64::INFINITY } else { macroquad::time::get_time() + 30. };
            self.plays.insert(id, end);
            Some(id)
        }
        fn set_volume(&mut self, id: u64, volume: f32) {
            unsafe { audio_playback_set_volume(id as u32, volume) }
        }
        fn stop(&mut self, id: u64) {
            self.plays.remove(&id);
            unsafe { audio_playback_stop(id as u32) }
        }
        fn playing(&self, id: u64) -> bool {
            self.plays.get(&id).is_some_and(|&end| macroquad::time::get_time() < end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Backend de prueba: anota lo que le piden.
    #[derive(Default)]
    struct Fake {
        next: u64,
        live: HashMap<u64, (String, f32, bool)>,
        log: Vec<String>,
    }

    impl Backend for Fake {
        fn has(&self, _: &str) -> bool {
            true
        }
        fn play(&mut self, file: &str, volume: f32, looped: bool) -> Option<u64> {
            self.next += 1;
            self.live.insert(self.next, (file.into(), volume, looped));
            self.log.push(format!("play {file} {volume:.2}"));
            Some(self.next)
        }
        fn set_volume(&mut self, id: u64, volume: f32) {
            if let Some(p) = self.live.get_mut(&id) {
                p.1 = volume;
            }
        }
        fn stop(&mut self, id: u64) {
            if let Some(p) = self.live.remove(&id) {
                self.log.push(format!("stop {}", p.0));
            }
        }
        fn playing(&self, id: u64) -> bool {
            self.live.contains_key(&id)
        }
    }

    fn vol_of(b: &Fake, file: &str) -> f32 {
        b.live.values().find(|p| p.0 == file).map(|p| p.1).unwrap_or(-1.)
    }

    #[test]
    fn rampa() {
        let mut r = Ramp::new(0., 1., 2.);
        assert_eq!(r.value(), 0.);
        r.advance(0.5);
        assert_eq!(r.value(), 0.25);
        r.advance(10.);
        assert_eq!(r.value(), 1.);
        assert!(r.done());
        r.retarget(0.5, 1.);
        assert_eq!(r.value(), 1.);
        r.advance(0.5);
        assert_eq!(r.value(), 0.75);
        assert_eq!(Ramp::new(0.3, 0.9, 0.).value(), 0.9, "sin duración: de inmediato");
    }

    #[test]
    fn canales_reemplazan() {
        let (mut m, mut b) = (Mixer::default(), Fake::default());
        let s = |f: &str, ch: &str| Cmd::PlaySound { file: f.into(), volume: 50., channel: ch.into() };
        m.command(&mut b, &s("a.wav", "voz"));
        m.command(&mut b, &s("b.wav", "voz"));
        m.command(&mut b, &s("c.wav", ""));
        m.command(&mut b, &s("c.wav", ""));
        assert_eq!(b.live.len(), 3, "en el canal queda solo el último; sin canal se superponen");
        assert_eq!(vol_of(&b, "b.wav"), 0.5);
        assert!(b.log.contains(&"stop a.wav".into()));
        m.command(&mut b, &Cmd::StopChannel { channel: "voz".into() });
        assert_eq!(b.live.len(), 2);
        m.command(&mut b, &Cmd::StopSound { file: "c.wav".into() });
        assert!(b.live.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn fundido_cruzado_de_musica() {
        let (mut m, mut b) = (Mixer::default(), Fake::default());
        m.command(&mut b, &Cmd::PlayMusic { file: "uno.ogg".into(), volume: 100., fade: 0. });
        assert_eq!(vol_of(&b, "uno.ogg"), 1.);
        m.command(&mut b, &Cmd::PlayMusic { file: "dos.ogg".into(), volume: 80., fade: 2. });
        assert_eq!(vol_of(&b, "dos.ogg"), 0., "entra desde cero");
        m.update(&mut b, 1.);
        assert!((vol_of(&b, "uno.ogg") - 0.5).abs() < 1e-5);
        assert!((vol_of(&b, "dos.ogg") - 0.4).abs() < 1e-5);
        m.update(&mut b, 1.);
        assert_eq!(vol_of(&b, "uno.ogg"), -1., "la anterior se apagó al terminar el fundido");
        assert!((vol_of(&b, "dos.ogg") - 0.8).abs() < 1e-5);
        // Sin fundido: corta de golpe.
        m.command(&mut b, &Cmd::PlayMusic { file: "tres.ogg".into(), volume: 100., fade: 0. });
        assert_eq!(b.live.len(), 1);
        // La misma música otra vez: no reinicia, solo cambia el volumen.
        let plays = b.log.iter().filter(|l| l.starts_with("play")).count();
        m.command(&mut b, &Cmd::PlayMusic { file: "tres.ogg".into(), volume: 50., fade: 0. });
        m.update(&mut b, 0.016);
        assert_eq!(b.log.iter().filter(|l| l.starts_with("play")).count(), plays);
        assert_eq!(vol_of(&b, "tres.ogg"), 0.5);
        m.command(&mut b, &Cmd::StopMusic);
        assert!(b.live.is_empty());
    }

    #[test]
    fn volumen_de_canal_con_fundido() {
        let (mut m, mut b) = (Mixer::default(), Fake::default());
        m.command(&mut b, &Cmd::PlayMusic { file: "m.ogg".into(), volume: 100., fade: 0. });
        m.command(&mut b, &Cmd::PlaySound { file: "lluvia.wav".into(), volume: 100., channel: "ambiente".into() });
        m.command(&mut b, &Cmd::ChannelVolume { channel: MUSIC.into(), volume: 0., fade: 4. });
        m.command(&mut b, &Cmd::ChannelVolume { channel: "ambiente".into(), volume: 25., fade: 0. });
        m.update(&mut b, 1.);
        assert!((vol_of(&b, "m.ogg") - 0.75).abs() < 1e-5);
        assert_eq!(vol_of(&b, "lluvia.wav"), 0.25);
        // Los sonidos nuevos del canal salen con el volumen del canal.
        m.command(&mut b, &Cmd::PlaySound { file: "trueno.wav".into(), volume: 100., channel: "ambiente".into() });
        assert_eq!(vol_of(&b, "trueno.wav"), 0.25);
        m.update(&mut b, 3.);
        assert_eq!(vol_of(&b, "m.ogg"), 0., "la música sigue sonando, en silencio");
        m.command(&mut b, &Cmd::StopChannel { channel: MUSIC.into() });
        assert_eq!(vol_of(&b, "m.ogg"), -1.);
        // Las órdenes que no son de sonido no se tocan.
        assert!(!m.command(&mut b, &Cmd::Quit));
    }
}
