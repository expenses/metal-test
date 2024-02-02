use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::macos::WindowExtMacOS,
};

use cocoa::appkit::NSView as _;
use core_graphics_types::geometry::CGSize;

use metal::objc::{rc::autoreleasepool, runtime::YES};

fn main() {
    // Create a window for viewing the content
    let event_loop = EventLoop::new();
    let size = winit::dpi::LogicalSize::new(800, 600);

    let window = winit::window::WindowBuilder::new()
        .with_inner_size(size)
        .with_title("Metal".to_string())
        .build(&event_loop)
        .unwrap();

    // Set up the GPU device found in the system
    let device = metal::Device::system_default().expect("no device found");
    println!("Your device is: {}", device.name(),);

    // Set the command queue used to pass commands to the device.
    let command_queue = device.new_command_queue();

    // Currently, MetalLayer is the only interface that provide
    // layers to carry drawable texture from GPU rendaring through metal
    // library to viewable windows.
    let layer = metal::MetalLayer::new();
    layer.set_device(&device);
    layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
    layer.set_presents_with_transaction(false);

    unsafe {
        let view = window.ns_view() as cocoa::base::id;
        view.setWantsLayer(YES);
        view.setLayer(
            layer.as_ref() as *const metal::MetalLayerRef as *mut metal::objc::runtime::Object
        );
    }

    let draw_size = window.inner_size();
    layer.set_drawable_size(CGSize::new(draw_size.width as f64, draw_size.height as f64));

    event_loop.run(move |event, _, control_flow| {
        autoreleasepool(|| {
            // ControlFlow::Wait pauses the event loop if no events are available to process.
            // This is ideal for non-game applications that only update in response to user
            // input, and uses significantly less power/CPU time than ControlFlow::Poll.
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    println!("The close button was pressed; stopping");
                    *control_flow = ControlFlow::Exit
                }
                Event::MainEventsCleared => {
                    // Queue a RedrawRequested event.
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    // It's preferrable to render in this event rather than in MainEventsCleared, since
                    // rendering in here allows the program to gracefully handle redraws requested
                    // by the OS.
                    let drawable = match layer.next_drawable() {
                        Some(drawable) => drawable,
                        None => return,
                    };

                    // Create a new command buffer for each render pass to the current drawable
                    let command_buffer = command_queue.new_command_buffer();

                    // Obtain a renderPassDescriptor generated from the view's drawable textures.
                    let render_pass_descriptor = metal::RenderPassDescriptor::new();
                    let color_attachment = render_pass_descriptor
                        .color_attachments()
                        .object_at(0)
                        .unwrap();

                    color_attachment.set_texture(Some(drawable.texture()));
                    color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                    color_attachment.set_clear_color(metal::MTLClearColor::new(0.5, 0.5, 0.8, 1.0));
                    color_attachment.set_store_action(metal::MTLStoreAction::Store);

                    let encoder = command_buffer.new_render_command_encoder(render_pass_descriptor);
                    encoder.end_encoding();

                    // Schedule a present once the framebuffer is complete using the current drawable.
                    command_buffer.present_drawable(drawable);

                    // Finalize rendering here & push the command buffer to the GPU.
                    command_buffer.commit();
                    command_buffer.wait_until_completed();
                }
                _ => (),
            }
        });
    });
}
