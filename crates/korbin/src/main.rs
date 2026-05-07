use regex::{Captures, Regex};
use ribir::prelude::*;
use korbin_core::{Editor, Mode, BufferType};
use korbin_core::highlighter::HighlightSpan;
use std::borrow::Cow;
use std::collections::HashMap;

fn get_highlight_color(name: Option<&str>) -> Color {
    match name {
        Some("keyword") => Color::from_rgb(86, 156, 214),
        Some("keyword.control") => Color::from_rgb(197, 134, 192),
        Some("function") | Some("function.method") | Some("function.macro") => Color::from_rgb(220, 220, 170),
        Some("type") | Some("type.builtin") | Some("markup.link") | Some("text.uri") => Color::from_rgb(78, 201, 176),
        Some("variable") | Some("variable.parameter") | Some("variable.builtin") | Some("constant") | Some("constant.builtin") => Color::from_rgb(156, 220, 254),
        Some("string") | Some("string.special") | Some("text.literal") | Some("markup.raw") => Color::from_rgb(206, 145, 120),
        Some("comment") | Some("markup.quote") | Some("text.quote") => Color::from_rgb(106, 153, 85),
        Some("punctuation") | Some("punctuation.bracket") | Some("punctuation.delimiter") | Some("punctuation.special") | Some("operator") | Some("text.list") | Some("markup.list") => Color::from_rgb(212, 212, 212),
        _ => Color::from_rgb(212, 212, 212),
    }
}

fn replace_vars_cmd_str(input: String, vars: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\{\{|}}|\{(?P<key>[^}]+)}").unwrap();

    re.replace_all(input.as_str(), |caps: &Captures| {
        if let Some(key) = caps.name("key") {
            vars.get(key.as_str()).unwrap_or(&"".to_string()).to_string()
        } else {
            match &caps[0] {
                "{{" => "{",
                "}}" => "}",
                _ => unreachable!(),
            }.to_string()
        }
    }).to_string()
}

#[derive(Declare, Query)]
pub struct SyntaxText {
  pub text: CowArc<str>,
  pub text_style: CowArc<TextStyle>,
  pub overflow: Overflow,
  pub spans: Vec<HighlightSpan>,
}

impl VisualText for SyntaxText {
  fn text(&self) -> CowArc<str> { self.text.clone() }
  fn text_style(&self) -> &TextStyle { &self.text_style }
  fn text_align(&self) -> TextAlign { TextAlign::Start }
  fn overflow(&self) -> Overflow { self.overflow }
}

impl Render for SyntaxText {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    self
      .text_layout(AppCtx::typography_store(), clamp.max)
      .visual_rect()
      .size
      .cast_unit()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let box_rect = Rect::from_size(ctx.box_size().unwrap());
    if ctx.painter().intersection_paint_bounds(&box_rect).is_none() {
      return;
    };

    let bounds = ctx.layout_clamp().map(|b| b.max).unwrap();
    let visual_glyphs = self.text_layout(AppCtx::typography_store(), bounds);
    let visual_rect = visual_glyphs.visual_rect();
    let font_db = AppCtx::font_db().clone();
    let font_size = self.text_style.font_size.into_pixel().value();
    let path_style = PathStyle::Fill;

    let mut current_color = None;
    let mut current_group = Vec::new();

    let mut painter = ctx.painter().save_guard();
    painter.translate(visual_rect.origin.x, visual_rect.origin.y);

    let mut glyphs: Vec<_> = visual_glyphs.glyph_bounds_in_rect(&Rect::new(Point::zero(), Size::new(10000., 10000.))).collect();
    glyphs.sort_by_key(|g| g.cluster);
    
    for g in glyphs {
        let color = self.get_color_for_cluster(g.cluster);
        if Some(color) != current_color {
            if !current_group.is_empty() {
                draw_glyphs(
                    &mut painter,
                    current_group.drain(..),
                    current_color.unwrap_or(Color::from_rgb(212, 212, 212)).into(),
                    font_size,
                    &path_style,
                    font_db.clone(),
                );
            }
            current_color = Some(color);
        }
        current_group.push(g);
    }
    
    if !current_group.is_empty() {
        draw_glyphs(
            &mut painter,
            current_group.drain(..),
            current_color.unwrap_or(Color::from_rgb(212, 212, 212)).into(),
            font_size,
            &path_style,
            font_db,
        );
    }
  }
}

impl SyntaxText {
    fn get_color_for_cluster(&self, cluster: u32) -> Color {
        let idx = self.spans.partition_point(|s| s.range.end <= (cluster as usize));
        if idx < self.spans.len() && self.spans[idx].range.contains(&(cluster as usize)) {
            return get_highlight_color(self.spans[idx].highlight_name.as_deref());
        }
        Color::from_rgb(212, 212, 212)
    }
}

fn main() {
    korbin_core::script::run_config();
    App::run(fn_widget! {
        let editor = Stateful::new(Editor::new());
        let mut layout_box = @LayoutBox { };
        let mut scroll_view = @ScrollableWidget { scrollable: Scrollable::Y };
        
        let text_style = TextStyle {
            font_size: FontSize::Pixel(22.0.into()),
            font_face: FontFace {
                families: Box::new([FontFamily::Name(Cow::Borrowed("IBM Plex Mono"))]),
                ..Default::default()
            },
            line_height: Some(Em::from_pixel(Pixel::from(26.4))),
            ..Default::default()
        };

        let text_style_kd = text_style.clone();
        let text_style_ch = text_style.clone();
        let text_style_exp = text_style.clone();

        @Column {
            auto_focus: true,
            background: Color::from_rgb(24, 24, 24),
            on_key_down: move |e| {
                let mut ed = $editor.write();
                if let PhysicalKey::Code(code) = e.key_code() {
                    match code {
                        KeyCode::Escape => {
                            if ed.mode == Mode::Visual {
                                ed.exit_visual();
                            } else {
                                if ed.mode == Mode::Insert {
                                    ed.push_history();
                                }
                                ed.mode = Mode::Normal;
                            }
                            ed.pending_operator = None;
                            ed.key_buffer.clear();
                        }
                        KeyCode::Backspace => {
                            if ed.mode == Mode::Insert {
                                ed.delete_char();
                            } else if ed.mode == Mode::Command {
                                ed.command_buffer.pop();
                            } else if ed.mode == Mode::Search {
                                ed.last_search.pop();
                            }
                        }
                        KeyCode::Tab => {
                            if ed.mode == Mode::Insert {
                                ed.insert_char('\t');
                            }
                        }
                        KeyCode::Enter => {
                            if ed.buffer_type == BufferType::Directory {
                                ed.trigger_action();
                            } else if ed.mode == Mode::Insert {
                                ed.insert_char('\n');
                            } else if ed.mode == Mode::Command {
                                let cmd_line = ed.command_buffer.trim().to_string();
                                let parts: Vec<&str> = cmd_line.split_whitespace().collect();
                                
                                if !parts.is_empty() {
                                    match parts[0] {
                                        "q" | "quit" => {
                                            std::process::exit(0);
                                        }
                                        "w" | "write" => {
                                            if parts.len() > 1 {
                                                let path = parts[1..].join(" ");
                                                match ed.save_as(path) {
                                                    Ok(_) => ed.status_message = Some("Saved".to_string()),
                                                    Err(e) => ed.status_message = Some(format!("Error: {}", e)),
                                                }
                                            } else {
                                                match ed.save() {
                                                    Ok(_) => ed.status_message = Some("Saved".to_string()),
                                                    Err(e) => ed.status_message = Some(format!("Error: {}", e)),
                                                }
                                            }
                                        }
                                        "e" | "edit" => {
                                            if parts.len() > 1 {
                                                let path = parts[1..].join(" ");
                                                match ed.open(path) {
                                                    Ok(_) => ed.status_message = Some("Loaded".to_string()),
                                                    Err(e) => ed.status_message = Some(format!("Error: {}", e)),
                                                }
                                            } else {
                                                ed.status_message = Some("Usage: :e <path>".to_string());
                                            }
                                        }
                                        _ => {
                                            ed.status_message = Some(format!("Unknown command: {}", parts[0]));
                                        }
                                    }
                                }
                                ed.mode = Mode::Normal;
                                ed.command_buffer.clear();
                            } else if ed.mode == Mode::Search {
                                let query = ed.last_search.clone();
                                let forward = ed.search_forward;
                                ed.perform_search(&query, forward);
                                ed.pending_operator = None;
                                ed.mode = Mode::Normal;
                            }
                        }
                        _ => {}
                    }
                }
                
                let width = $layout_box.layout_width() - 20.0;
                let text_widget = Text {
                    text: ed.display_text().into(),
                    foreground: Color::BLACK.into(),
                    text_style: text_style_kd.clone().into(),
                    path_style: PathStyle::Fill,
                    overflow: Overflow::AutoWrap,
                    text_align: TextAlign::Start,
                };
                let gs = text_widget.text_layout(AppCtx::typography_store(), Size::new(width.max(0.), f32::INFINITY));
                let pos = ed.get_cursor_pos();
                let byte_pos = ed.text.char_to_byte(pos);
                let display_byte_pos = ed.rope_byte_to_display_byte(byte_pos);
                let (row, col) = gs.position_by_cluster(display_byte_pos);
                let rect = gs.glyph_rect(row, col);
                let cursor_y = rect.min_y();
                let cursor_bottom = rect.max_y();
                let view_top = -$scroll_view.scroll_pos.y;
                let view_height = $scroll_view.scroll_view_size().height;
                let view_bottom = view_top + view_height - 26.4;
                let mut new_view_top = view_top;
                
                if cursor_y < view_top {
                    new_view_top = cursor_y;
                } else if cursor_bottom > view_bottom && view_height > 0.0 {
                    new_view_top = cursor_bottom - view_height + 26.4;
                }
                
                if new_view_top != view_top {
                    let current_x = $scroll_view.scroll_pos.x;
                    $scroll_view.write().jump_to(Point::new(current_x, -new_view_top));
                }
            },
            on_chars: move |e| {
                let mut ed = $editor.write();
                for c in e.chars.chars() {
                    if c.is_control() { continue; }
                    match ed.mode {
                        Mode::Normal | Mode::Visual => {
                            if ed.waiting_for_register_name {
                                ed.selected_register = c;
                                ed.waiting_for_register_name = false;
                                continue;
                            }
                            if c == '"' && ed.key_buffer.is_empty() {
                                ed.waiting_for_register_name = true;
                                continue;
                            }
                            
                            ed.key_buffer.push(c);
                            let key = ed.key_buffer.clone();
                            let registry = korbin_core::REGISTRY.lock().unwrap();

                            let mut action_to_run = registry.get_action(ed.mode, &ed.buffer_type, &ed.file_type, &key);
                            let mut is_pref = registry.is_prefix(ed.mode, &ed.buffer_type, &ed.file_type, &key);

                            if action_to_run.is_none() && !is_pref {
                                ed.key_buffer.clear();
                                ed.key_buffer.push(c);
                                let key2 = ed.key_buffer.clone();
                                action_to_run = registry.get_action(ed.mode, &ed.buffer_type, &ed.file_type, &key2);
                                is_pref = registry.is_prefix(ed.mode, &ed.buffer_type, &ed.file_type, &key2);
                                
                                if action_to_run.is_none() && !is_pref {
                                    ed.key_buffer.clear();
                                    ed.pending_operator = None;
                                }
                            }

                            if let Some(action) = action_to_run {
                                ed.key_buffer.clear();
                                match action {
                                    korbin_core::Action::EditorCommand(cmd) => ed.dispatch_command(&cmd),
                                    korbin_core::Action::ShellCommand { cmd, async_exec } => {
                                        let mut values = HashMap::new();
                                        values.insert("line".to_string(), (ed.cursor_line + 1).to_string());
                                        values.insert("col".to_string(), (ed.cursor_col + 1).to_string());
                                        values.insert("file".to_string(), ed.file_path.as_deref().unwrap_or("").to_string());
                                        let final_cmd = replace_vars_cmd_str(cmd.to_string(), &values);
                                        if async_exec {
                                            let cmd_clone = final_cmd.clone();
                                            std::thread::spawn(move || {
                                                let _ = std::process::Command::new("sh").arg("-c").arg(cmd_clone).output();
                                            });
                                            ed.status_message = Some(format!("Started async: {}", final_cmd));
                                        } else {
                                            match std::process::Command::new("sh").arg("-c").arg(&final_cmd).output() {
                                                Ok(output) => {
                                                    let out = String::from_utf8_lossy(&output.stdout);
                                                    ed.status_message = Some(format!("Cmd output: {}", out.trim()));
                                                }
                                                Err(e) => ed.status_message = Some(format!("Cmd error: {}", e)),
                                            }
                                        }
                                        ed.pending_operator = None;
                                    }
                                }
                            }
                            
                            if ed.mode == Mode::Visual {
                                ed.update_selection();
                            }
                        }
                        Mode::Insert => {
                            ed.insert_char(c);
                        }
                        Mode::Command => {
                            ed.command_buffer.push(c);
                        },
                        Mode::Search => {
                            ed.last_search.push(c);
                        }
                    }
                }
                
                let width = $layout_box.layout_width() - 20.0;
                let text_widget = Text {
                    text: ed.display_text().into(),
                    foreground: Color::BLACK.into(),
                    text_style: text_style_ch.clone().into(),
                    path_style: PathStyle::Fill,
                    overflow: Overflow::AutoWrap,
                    text_align: TextAlign::Start,
                };
                let gs = text_widget.text_layout(AppCtx::typography_store(), Size::new(width.max(0.), f32::INFINITY));
                let pos = ed.get_cursor_pos();
                let byte_pos = ed.text.char_to_byte(pos);
                let display_byte_pos = ed.rope_byte_to_display_byte(byte_pos);
                let (row, col) = gs.position_by_cluster(display_byte_pos);
                let rect = gs.glyph_rect(row, col);
                let cursor_y = rect.min_y();
                let cursor_bottom = rect.max_y();
                let view_top = -$scroll_view.scroll_pos.y;
                let view_height = $scroll_view.scroll_view_size().height;
                let view_bottom = view_top + view_height - 26.4;
                let mut new_view_top = view_top;
                
                if cursor_y < view_top {
                    new_view_top = cursor_y;
                } else if cursor_bottom > view_bottom && view_height > 0.0 {
                    new_view_top = cursor_bottom - view_height + 26.4;
                }
                
                if new_view_top != view_top {
                    let current_x = $scroll_view.scroll_pos.x;
                    $scroll_view.write().jump_to(Point::new(current_x, -new_view_top));
                }
            },
            @Expanded {
                @ $scroll_view {
                    @ $layout_box {
                        @Stack {
                            padding: EdgeInsets::all(10.),
                            @ {
                            let _text_style = text_style_exp.clone();
                            pipe! {
                                let _ = $editor.version;
                                let width = $layout_box.layout_width() - 20.0;
                                let text = $editor.display_text();
                                let text_widget = Text {
                                    text: text.into(),
                                    foreground: Color::BLACK.into(),
                                    text_style: text_style_exp.clone().into(),
                                    path_style: PathStyle::Fill,
                                    overflow: Overflow::AutoWrap,
                                    text_align: TextAlign::Start,
                                };
                                text_widget.text_layout(AppCtx::typography_store(), Size::new(width.max(0.), f32::INFINITY))
                            }.map(move |gs| {
                                let mut overlays = Vec::new();

                                // Selection
                                if let Some((s, safe_e)) = $editor.get_selection_char_bounds() {
                                    let s_byte = $editor.text.char_to_byte(s);
                                    let e_byte = $editor.text.char_to_byte(safe_e);
                                    let display_s_byte = $editor.rope_byte_to_display_byte(s_byte);
                                    let display_e_byte = $editor.rope_byte_to_display_byte(e_byte);
                                    let range = display_s_byte..display_e_byte;
                                    for rect in gs.select_range(&range) {
                                        overlays.push(@Container {
                                            margin: EdgeInsets {
                                                top: rect.min_y(),
                                                left: rect.min_x(),
                                                ..Default::default()
                                            },
                                            size: rect.size,
                                            background: Color::from_rgb(100, 200, 255).with_alpha(0.3),
                                        });
                                    }
                                }

                                // Cursor
                                let pos = $editor.get_cursor_pos();
                                let byte_pos = $editor.text.char_to_byte(pos);
                                let display_byte_pos = $editor.rope_byte_to_display_byte(byte_pos);
                                let (row, col) = gs.position_by_cluster(display_byte_pos);
                                let rect = gs.glyph_rect(row, col);

                                overlays.push(@Container {
                                    margin: EdgeInsets {
                                        top: rect.min_y(),
                                        left: rect.min_x(),
                                        ..Default::default()
                                    },
                                    background: match $editor.mode {
                                        Mode::Normal => Color::from_rgb(100, 100, 255).with_alpha(0.5),
                                        Mode::Insert => Color::from_rgb(255, 255, 255).with_alpha(0.8),
                                        Mode::Visual => Color::from_rgb(255, 200, 100).with_alpha(0.5),
                                        _ => Color::from_rgb(200, 200, 200).with_alpha(0.5),
                                    },
                                    size: Size::new(
                                        if $editor.mode == Mode::Insert { 2.0 } else { rect.width().max(1.0) },
                                        rect.height().max(1.0)
                                    ),
                                });

                                overlays
                            })
                        }

                        @SyntaxText {
                            text: pipe! {
                                let _ = $editor.version;
                                CowArc::from($editor.display_text())
                            },
                            text_style: text_style.clone(),
                            overflow: Overflow::AutoWrap,
                            spans: pipe! {
                                let _ = $editor.version;
                                $editor.get_display_highlight_spans()
                            },
                        }
                    }
                }
            }
        }
        @Container {
                size: Size::new(f32::INFINITY, 26.4),
                background: Color::from_rgb(40, 40, 40),
                padding: EdgeInsets::symmetrical(0., 10.),
                @Row {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: Align::Center,
                    @Text {
                        text: pipe! {
                            let _ = $editor.version;
                            let _ = $editor.command_buffer;
                            let bt_name = $editor.buffer_type.name().to_uppercase();
                            let ft_name = $editor.file_type.to_uppercase();
                            let ft_name_sanitized = if !(ft_name == bt_name) { " [".to_string() + &ft_name + "]" } else  { "".to_string() };
                            let buffer_info = format!("{}{}", bt_name, ft_name_sanitized);
                            let search_prefix = if $editor.search_forward {  "/".to_string() } else { "?".to_string() };
                            let mode_info = match $editor.mode {
                                Mode::Normal => "NORMAL".to_string(),
                                Mode::Insert => "INSERT".to_string(),
                                Mode::Visual => "VISUAL".to_string(),
                                Mode::Command => ":".to_string() + &$editor.command_buffer,
                                Mode::Search => search_prefix + &$editor.last_search
                            };
                            format!(" {} | {}", buffer_info, mode_info)
                        },
                        foreground: Color::from_rgb(212, 212, 212),
                        text_style: TextStyle {
                            font_size: FontSize::Pixel(20.0.into()),
                            ..text_style.clone()
                        },
                    }
                    @Text {
                        text: pipe! {
                            let _ = $editor.version;
                            let _ = $editor.command_buffer;
                            let status = $editor.status_message.clone().unwrap_or_default();
                            format!("{} ", status)
                        },
                        foreground: Color::from_rgb(200, 200, 200),
                        text_style: TextStyle {
                            font_size: FontSize::Pixel(18.0.into()),
                            ..text_style.clone()
                        },
                    }
                }
            }
        }
    });
}
