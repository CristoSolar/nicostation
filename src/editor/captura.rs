//! `--captura salida.png`: dibuja el editor sin ventana (wgpu fuera de pantalla) y guarda una imagen.
//! Sirve para revisar la interfaz cuando no hay escritorio a mano (pantalla bloqueada, CI).
//! `--tecla <nombre>` aprieta una tecla de egui (p. ej. `Enter`) a mitad de la captura.

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use std::path::Path;

const W: u32 = 1360;
const H: u32 = 820;
const CUADROS: u32 = 30;

/// Espera un futuro de wgpu (en escritorio ya vienen resueltos o se resuelven al hacer `poll`).
fn esperar<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

pub fn capturar(app: &mut impl eframe::App, ctx: &egui::Context, tecla: Option<egui::Key>, salida: &Path) -> Result<(), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = esperar(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).map_err(|e| e.to_string())?;
    let (device, queue) = esperar(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|e| e.to_string())?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut renderer = egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());
    let size = wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("captura"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    let mut frame = eframe::Frame::_new_kittest();
    let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [W, H], pixels_per_point: 1. };
    let mut jobs = vec![];
    for n in 0..CUADROS {
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(W as f32, H as f32))),
            time: Some(n as f64 / 30.),
            predicted_dt: 1. / 30.,
            focused: true,
            ..Default::default()
        };
        if let Some(k) = tecla.filter(|_| n == CUADROS / 2) {
            for pressed in [true, false] {
                raw.events.push(egui::Event::Key { key: k, physical_key: None, pressed, repeat: false, modifiers: Default::default() });
            }
        }
        let out = ctx.run_ui(raw, |ui| app.ui(ui, &mut frame));
        for (id, ds) in &out.textures_delta.set {
            for d in ds {
                renderer.update_texture(&device, &queue, *id, d);
            }
        }
        jobs = ctx.tessellate(out.shapes, out.pixels_per_point);
        for id in &out.textures_delta.free {
            renderer.free_texture(id);
        }
    }
    let mut enc = device.create_command_encoder(&Default::default());
    let cmds = renderer.update_buffers(&device, &queue, &mut enc, &jobs, &screen);
    {
        let pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("captura"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view, depth_slice: None, resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
    }
    let fila = (W * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size: (fila * H) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false,
    });
    enc.copy_texture_to_buffer(
        tex.as_image_copy(),
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(fila), rows_per_image: Some(H) } },
        size,
    );
    queue.submit(cmds.into_iter().chain([enc.finish()]));
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
    let datos = buf.slice(..).get_mapped_range().map_err(|e| format!("{e:?}"))?;
    let mut img = image::RgbaImage::new(W, H);
    for y in 0..H {
        let a = (y * fila) as usize;
        img.as_mut()[(y * W * 4) as usize..((y + 1) * W * 4) as usize].copy_from_slice(&datos[a..a + (W * 4) as usize]);
    }
    img.save(salida).map_err(|e| e.to_string())
}
