
use crate::{
	resource_pack::ResourcePack,
	localization::Localization,
	gui::{
		context::{
			GuiGlobalContext,
			GuiSpatialContext,
			GuiWindowContext,
			FocusLevel,
		},
		event::ScrolledAmount,
		state_frame_obj::GuiStateFrameObj,
	},
};
use graphics::{
	Renderer,
	frame_content::FrameContent,
};
use std::{
	collections::HashSet,
	sync::{
		Arc,
		RwLock,
	},
};
use winit::{
    event_loop::{
    	ControlFlow,
    	EventLoop,
    },
    window::{
    	Window,
    	WindowBuilder,
    },
    event::{
    	Event,
    	WindowEvent,
    	DeviceEvent,
    	VirtualKeyCode,
	    ScanCode,
	    MouseButton,
	    ElementState,
	    MouseScrollDelta
	},
};
use pollster::FutureExt;
use vek::*;


struct State {
    renderer: RwLock<Renderer>,
    resources: ResourcePack,
    lang: Localization,
    focus_level: FocusLevel,
	pressed_keys_semantic: HashSet<VirtualKeyCode>,
    pressed_keys_physical: HashSet<ScanCode>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    cursor_pos: Option<Vec2<f32>>,
    size: Extent2<u32>,
    scale: f32,

    // TODO: only necessary until they stabilize const set constructor
    pressed_keys_semantic_empty: HashSet<VirtualKeyCode>,
    pressed_keys_physical_empty: HashSet<ScanCode>,
    pressed_mouse_buttons_empty: HashSet<MouseButton>,
}

impl State {
	fn new(
		window: &Window,
		renderer: Renderer,
		resources: ResourcePack,
		lang: Localization,
	) -> Self
	{
		let winit_size = window.inner_size();
		State {
			renderer: RwLock::new(renderer),
			resources,
			lang,
			focus_level: FocusLevel::Focused,
			pressed_keys_semantic: HashSet::new(),
			pressed_keys_physical: HashSet::new(),
			pressed_mouse_buttons: HashSet::new(),
			cursor_pos: None,
			size: Extent2 {
				w: winit_size.width,
				h: winit_size.height,
			},
			scale: window.scale_factor() as f32,

			pressed_keys_semantic_empty: HashSet::new(),
			pressed_keys_physical_empty: HashSet::new(),
			pressed_mouse_buttons_empty: HashSet::new(),
		}
	}

	fn with_ctx<F>(&self, f: F)
	where
		F: FnOnce(&GuiWindowContext),
	{
		f(&GuiWindowContext {
			spatial: GuiSpatialContext {
				global: &GuiGlobalContext {
					renderer: &self.renderer,
					resources: &self.resources,
					lang: &self.lang,
					focus_level: self.focus_level,
					pressed_keys_semantic:
						if self.focus_level >= FocusLevel::Focused {
							&self.pressed_keys_semantic
						} else { &self.pressed_keys_semantic_empty },
					pressed_keys_physical: 
						if self.focus_level >= FocusLevel::Focused {
							&self.pressed_keys_physical
						} else { &self.pressed_keys_physical_empty },
					pressed_mouse_buttons: 
						if self.focus_level >= FocusLevel::Focused {
							&self.pressed_mouse_buttons
						} else { &self.pressed_mouse_buttons_empty },
				},
				cursor_pos: self.cursor_pos,
			},
			size: self.size,
			scale: self.scale,
		})
	}
}


struct Stack(Vec<Box<dyn GuiStateFrameObj>>);

impl Stack {
	fn new(state_frame: Box<dyn GuiStateFrameObj>) -> Self {
		Stack(vec![state_frame])
	}

	fn top(&mut self) -> &mut dyn GuiStateFrameObj {
		&mut **self.0.iter_mut().rev().next().unwrap()
	}
}

pub struct GuiEventLoop {
	event_loop: EventLoop<()>,
	window: Arc<Window>,
	pub renderer: Renderer,
}

impl GuiEventLoop {
	pub fn new() -> Self {
		let event_loop = EventLoop::new();
		let window = WindowBuilder::new()
			.build(&event_loop)
			.expect("failed to build window");
		let window = Arc::new(window);

		let renderer = Renderer::new(Arc::clone(&window))
			.block_on()
			.expect("failed to create renderer");
		
		GuiEventLoop {
			event_loop,
			window,
			renderer,
		}		
	}

	pub fn run(
		self,
		state_frame: Box<dyn GuiStateFrameObj>,
		resources: ResourcePack,
		lang: Localization,
	) -> ! {
		let mut stack = Stack::new(state_frame);
		let mut state = State::new(
			&self.window,
			self.renderer,
			resources,
			lang,
		);

		self.event_loop.run(move |event, _target, control_flow| match event {
			Event::WindowEvent { event, .. } => match event {
				WindowEvent::Resized(winit_size) => {
					state.size.w = winit_size.width;
					state.size.h = winit_size.height;

					state.renderer.try_write().unwrap().resize(state.size);
				}
				WindowEvent::CloseRequested => {
					todo!()
				}
				WindowEvent::Destroyed => {
					todo!()
				}
				WindowEvent::ReceivedCharacter(c) => {
					state.with_ctx(|ctx| stack
						.top()
						.on_character_input(ctx, c));
				}
				WindowEvent::Focused(focused) => {
					state.focus_level =
						if focused { FocusLevel::Focused }
						else { FocusLevel::Unfocused };
					state.with_ctx(|ctx| stack.top().on_focus_change(ctx));
				}
				WindowEvent::KeyboardInput {
					is_synthetic: false,
					input,
					..
				} => {
					let focused = state.focus_level >= FocusLevel::Focused;
					match input.state {
						ElementState::Pressed => {
							// semantic press
							if let Some(key) = input.virtual_keycode {
								let changed = state
									.pressed_keys_semantic
									.insert(key);
								if changed && focused {
									state.with_ctx(|ctx| stack
										.top()
										.on_key_press_semantic(ctx, key));
								}
							}

							// physical press
							let key = input.scancode;
							let changed = state
								.pressed_keys_physical
								.insert(key);
							if changed && focused {
								state.with_ctx(|ctx| stack
									.top()
									.on_key_press_physical(ctx, key));
							}
						}
						ElementState::Released => {
							// semantic release
							if let Some(key) = input.virtual_keycode {
								let changed = state
									.pressed_keys_semantic
									.remove(&key);
								if changed && focused {
									state.with_ctx(|ctx| stack
										.top()
										.on_key_press_semantic(ctx, key));
								}
							}

							// physical release
							let key = input.scancode;
							let changed = state
								.pressed_keys_physical
								.remove(&key);
							if changed && focused {
								state.with_ctx(|ctx| stack
									.top()
									.on_key_press_physical(ctx, key));
							}
						}
					}
				}
				WindowEvent::CursorMoved { position, .. } => {
					state.cursor_pos = Some(Vec2 {
						x: position.x as f32,
						y: position.y as f32,
					});
					if state.focus_level < FocusLevel::MouseCaptured {
						state.with_ctx(|ctx| stack.top().on_cursor_move(ctx));
					}
				}
				WindowEvent::MouseWheel { delta, .. } => {
					let amount = match delta {
						MouseScrollDelta::LineDelta(
							x,
							y,
						) => ScrolledAmount::Lines(Vec2 { x, y }),
						MouseScrollDelta::PixelDelta(
							pos,
						) => ScrolledAmount::Pixels(Vec2 {
							x: pos.x as f32,
							y: pos.y as f32,
						}),
					};
					match state.focus_level {
						FocusLevel::Unfocused => (),
						FocusLevel::Focused => {
							state.with_ctx(|ctx| stack
								.top()
								.on_cursor_scroll(ctx, amount));
						}
						FocusLevel::MouseCaptured => {
							state.with_ctx(|ctx| stack
								.top()
								.on_captured_mouse_scroll(ctx, amount));
						}
					}
				}
				WindowEvent::MouseInput {
					state: element_state,
					button,
					..
				} => match element_state {
					ElementState::Pressed => {
						let changed = state
							.pressed_mouse_buttons
							.insert(button);
						if changed {
							match state.focus_level {
								FocusLevel::Unfocused => (),
								FocusLevel::Focused => {
									state.with_ctx(|ctx| stack
										.top()
										.on_cursor_click(ctx, button));
								}
								FocusLevel::MouseCaptured => {
									state.with_ctx(|ctx| stack
										.top()
										.on_captured_mouse_click(ctx, button));
								}
							}
						}
					}
					ElementState::Released => {
						let changed = state
							.pressed_mouse_buttons
							.remove(&button);
						if changed {
							match state.focus_level {
								FocusLevel::Unfocused => (),
								FocusLevel::Focused => {
									state.with_ctx(|ctx| stack
										.top()
										.on_cursor_release(ctx, button));
								}
								FocusLevel::MouseCaptured => {
									state.with_ctx(|ctx| stack
										.top()
										.on_captured_mouse_release(
											ctx,
											button,
										));
								}
							}
						}
					}
				}
				WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
					state.scale = scale_factor as f32;
				}
				_ => (),
			}
			Event::DeviceEvent { event, .. } => match event {
				DeviceEvent::MouseMotion { delta: (x, y) } => {
					if state.focus_level == FocusLevel::MouseCaptured {
						state.with_ctx(|ctx| stack
							.top()
							.on_captured_mouse_move(
								ctx,
								Vec2 { x: x as f32, y: y as f32 },
							));
					}
				}
				_ => (),
			}
			Event::MainEventsCleared => state.with_ctx(|ctx| {
				let mut frame_content = FrameContent::new();
				stack.top().draw(ctx, &mut frame_content);
				println!("{}", frame_content.to_pseudo_xml());
				ctx.spatial.global.renderer
					.try_write()
					.unwrap()
					.draw_frame(&frame_content)
					.expect("failed to draw frame");
			}),
			Event::RedrawEventsCleared => {
				*control_flow = ControlFlow::Poll;
			}
			Event::LoopDestroyed => {
				todo!()
			}
			_ => (),
		});
	}
}
