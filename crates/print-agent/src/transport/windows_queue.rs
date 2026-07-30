//! Windows print-queue transport: raw spooler write via `winspool.drv` for
//! thermal printers installed as a generic/text-only Windows queue (matching
//! POS_system's existing Rust agent's approach, see docs/ARCHITECTURE.md),
//! plus real GDI-based silent printing (`print_document_gdi`) for normal
//! (A4/A3/Letter) printers -- closing what was previously a `NotImplemented`
//! gap in `print_silent`.
//!
//! Verified as far as this environment allows: this crate is developed on
//! Linux, so none of this has ever *run* against a real Windows printer --
//! but it does successfully cross-compile AND link into a real Windows PE32+
//! binary via `cargo build --target x86_64-pc-windows-gnu` (mingw-w64 is
//! available here), which is checked as part of this crate's test/build
//! process. That catches real API-mismatch bugs (and did: the original raw
//! spooler code below referenced `OpenPrinterW` without windows-sys's
//! `Win32_Graphics_Gdi` feature enabled, which doesn't compile) even though
//! it can't catch runtime/hardware issues. There is no Wine available here
//! either, so nothing has actually executed.

use crate::error::AgentError;

#[cfg(target_os = "windows")]
pub async fn send_raw(printer_name: &str, bytes: Vec<u8>) -> Result<(), AgentError> {
    let printer_name = printer_name.to_string();
    tokio::task::spawn_blocking(move || send_raw_blocking(&printer_name, &bytes))
        .await
        .map_err(|e| AgentError::TransportError(format!("Windows spooler task panicked: {e}")))?
}

#[cfg(target_os = "windows")]
fn send_raw_blocking(printer_name: &str, bytes: &[u8]) -> Result<(), AgentError> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Graphics::Printing::{
        ClosePrinter, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterW, StartPagePrinter, WritePrinter, DOC_INFO_1W,
    };

    let wide_name: Vec<u16> = printer_name.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_datatype: Vec<u16> = "RAW\0".encode_utf16().collect();
    let wide_doc_name: Vec<u16> = "PrintersSDK Job\0".encode_utf16().collect();

    unsafe {
        let mut handle: HANDLE = null_mut();
        if OpenPrinterW(wide_name.as_ptr() as *mut _, &mut handle, null_mut()) == 0 {
            return Err(AgentError::TransportError(format!("OpenPrinter failed for '{printer_name}'")));
        }

        let mut doc_info = DOC_INFO_1W {
            pDocName: wide_doc_name.as_ptr() as *mut _,
            pOutputFile: null_mut(),
            pDatatype: wide_datatype.as_ptr() as *mut _,
        };
        let job_id = StartDocPrinterW(handle, 1, &mut doc_info as *mut _ as *mut _);
        if job_id == 0 {
            ClosePrinter(handle);
            return Err(AgentError::TransportError("StartDocPrinter failed".to_string()));
        }
        if StartPagePrinter(handle) == 0 {
            EndDocPrinter(handle);
            ClosePrinter(handle);
            return Err(AgentError::TransportError("StartPagePrinter failed".to_string()));
        }

        let mut written: u32 = 0;
        let ok = WritePrinter(handle, bytes.as_ptr() as *mut _, bytes.len() as u32, &mut written);

        EndPagePrinter(handle);
        EndDocPrinter(handle);
        ClosePrinter(handle);

        if ok == 0 || written as usize != bytes.len() {
            return Err(AgentError::TransportError("WritePrinter did not write the full document".to_string()));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub async fn send_raw(_printer_name: &str, _bytes: Vec<u8>) -> Result<(), AgentError> {
    Err(AgentError::NotImplemented("the Windows print-queue transport only runs on Windows".to_string()))
}

/// Silent (no-dialog) print of a PrintDocument to a normal (a4/a3/letter)
/// Windows printer via GDI -- `CreateDC` + `StartDoc`/`StartPage` + GDI
/// drawing calls, the same subsystem every Windows printer driver ultimately
/// implements against. This draws the document directly rather than going
/// through PDF (unlike the CUPS path in print_silent), so it needs no
/// bundled PDF-rasterization engine.
#[cfg(target_os = "windows")]
pub async fn print_document_gdi(printer_name: &str, doc: &printers_core::PrintDocument, copies: u32) -> Result<(), AgentError> {
    let printer_name = printer_name.to_string();
    let doc = doc.clone();
    tokio::task::spawn_blocking(move || print_document_gdi_blocking(&printer_name, &doc, copies))
        .await
        .map_err(|e| AgentError::TransportError(format!("Windows GDI print task panicked: {e}")))?
}

#[cfg(not(target_os = "windows"))]
pub async fn print_document_gdi(_printer_name: &str, _doc: &printers_core::PrintDocument, _copies: u32) -> Result<(), AgentError> {
    Err(AgentError::NotImplemented("the Windows GDI print transport only runs on Windows".to_string()))
}

/// Builds a DEVMODEW (as a raw byte buffer, since its trailing driver-private
/// extra data makes it variably sized) requesting the paper size that matches
/// `doc.paper.kind`, by fetching the printer's default DEVMODE via
/// `DocumentPropertiesW` and overwriting just the paper fields. Without this,
/// `CreateDCW`'s `lpInitData` would be null and every job would silently
/// print on the driver's default paper size (typically Letter) regardless of
/// what the document declared -- an A4 document would come out on Letter
/// paper with no error. Returns `None` (falls back to the driver default) if
/// any step fails, rather than aborting the print job over a paper-size
/// preference.
#[cfg(target_os = "windows")]
fn devmode_for_paper(printer_name: &[u16], paper: &printers_core::types::Paper, width_mm: f64, height_mm: f64) -> Option<Vec<u8>> {
    use printers_core::types::PaperKind;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Graphics::Gdi::{DEVMODEW, DMPAPER_A3, DMPAPER_A4, DMPAPER_LETTER, DM_IN_BUFFER, DM_OUT_BUFFER, DM_PAPERLENGTH, DM_PAPERSIZE, DM_PAPERWIDTH};
    use windows_sys::Win32::Graphics::Printing::{ClosePrinter, DocumentPropertiesW, OpenPrinterW};

    unsafe {
        let mut hprinter: HANDLE = null_mut();
        if OpenPrinterW(printer_name.as_ptr() as *mut _, &mut hprinter, null_mut()) == 0 {
            return None;
        }

        let size = DocumentPropertiesW(null_mut(), hprinter, printer_name.as_ptr(), null_mut(), null_mut(), 0);
        if size <= 0 {
            ClosePrinter(hprinter);
            return None;
        }

        let mut buf = vec![0u8; size as usize];
        let devmode_ptr = buf.as_mut_ptr() as *mut DEVMODEW;
        let got = DocumentPropertiesW(null_mut(), hprinter, printer_name.as_ptr(), devmode_ptr, null_mut(), DM_OUT_BUFFER);
        if got < 0 {
            ClosePrinter(hprinter);
            return None;
        }

        let dm = &mut *devmode_ptr;
        match paper.kind {
            PaperKind::A4 => {
                dm.dmFields |= DM_PAPERSIZE;
                dm.Anonymous1.Anonymous1.dmPaperSize = DMPAPER_A4 as i16;
            }
            PaperKind::A3 => {
                dm.dmFields |= DM_PAPERSIZE;
                dm.Anonymous1.Anonymous1.dmPaperSize = DMPAPER_A3 as i16;
            }
            PaperKind::Letter => {
                dm.dmFields |= DM_PAPERSIZE;
                dm.Anonymous1.Anonymous1.dmPaperSize = DMPAPER_LETTER as i16;
            }
            PaperKind::Custom | PaperKind::Thermal58 | PaperKind::Thermal80 => {
                dm.dmFields |= DM_PAPERWIDTH | DM_PAPERLENGTH;
                dm.Anonymous1.Anonymous1.dmPaperWidth = (width_mm * 10.0).round() as i16;
                dm.Anonymous1.Anonymous1.dmPaperLength = (height_mm * 10.0).round() as i16;
            }
        }

        // Let the driver validate/merge our requested fields back into the buffer.
        DocumentPropertiesW(null_mut(), hprinter, printer_name.as_ptr(), devmode_ptr, devmode_ptr, DM_IN_BUFFER | DM_OUT_BUFFER);
        ClosePrinter(hprinter);
        Some(buf)
    }
}

#[cfg(target_os = "windows")]
fn print_document_gdi_blocking(printer_name: &str, doc: &printers_core::PrintDocument, copies: u32) -> Result<(), AgentError> {
    use printers_core::render::barcode::generate_barcode_bits;
    use printers_core::render::qr::qr_modules;
    use printers_core::render::text_shaping::needs_shaping;
    use printers_core::types::{resolve_margin_mm, resolve_page_height_mm, resolve_page_width_mm, Align, PrintNode, QrEcLevel};
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Graphics::GdiPlus::{
        GdipCreateFont, GdipCreateFontFamilyFromName, GdipCreateFromHDC, GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteFont, GdipDeleteFontFamily,
        GdipDeleteGraphics, GdipDrawString, GdipMeasureString, GdiplusShutdown, GdiplusStartup, FontStyleBold, FontStyleRegular, GpFont, GpFontFamily,
        GpGraphics, GpSolidFill, RectF, UnitPixel,
    };
    use windows_sys::Win32::Storage::Xps::{EndDoc, EndPage, StartDocW, StartPage, DOCINFOW};

    const FONT_SIZE_PT: f64 = 11.0;
    const LINE_HEIGHT_FACTOR: f64 = 1.4;

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // SAFETY: every GDI/Xps call below operates on `hdc`, a valid HDC for the
    // lifetime of this function (created just above, deleted at the end via
    // the `result` closure's cleanup), or on GDI objects (fonts) created and
    // destroyed within the same iteration -- standard GDI object lifetime
    // discipline (CreateX/SelectObject old-back/DeleteObject).
    unsafe {
        let width_mm = resolve_page_width_mm(&doc.paper)?;
        let height_mm = resolve_page_height_mm(&doc.paper).unwrap_or(297.0);
        let margin_mm = resolve_margin_mm(&doc.paper);

        let wide_printer_name = to_wide(printer_name);
        let devmode_buf = devmode_for_paper(&wide_printer_name, &doc.paper, width_mm, height_mm);
        let devmode_ptr = devmode_buf.as_ref().map(|b| b.as_ptr() as *const DEVMODEW).unwrap_or(null_mut());
        let hdc = CreateDCW(null_mut(), wide_printer_name.as_ptr(), null_mut(), devmode_ptr);
        if hdc.is_null() {
            return Err(AgentError::TransportError(format!("CreateDC failed for printer '{printer_name}'")));
        }

        // GDI+ (via GdipDrawString), not raw GDI's TextOutW, is what actually
        // applies Myanmar's complex-script shaping (vowel-sign/medial
        // reordering) when drawing text -- TextOutW draws one glyph per
        // codepoint in string order regardless of font, which is wrong for
        // Myanmar. Started once per print job, paired with GdiplusShutdown
        // in the cleanup below; only used for text needing it (see
        // draw_myanmar_text), plain ASCII keeps using the existing GDI path.
        let mut gdiplus_token: usize = 0;
        let startup_input = windows_sys::Win32::Graphics::GdiPlus::GdiplusStartupInput { GdiplusVersion: 1, DebugEventCallback: 0, SuppressBackgroundThread: 0, SuppressExternalCodecs: 0 };
        let mut startup_output = std::mem::zeroed();
        GdiplusStartup(&mut gdiplus_token, &startup_input, &mut startup_output);
        let mut gdiplus_graphics: *mut GpGraphics = null_mut();
        GdipCreateFromHDC(hdc, &mut gdiplus_graphics);

        let dpi_x = GetDeviceCaps(hdc, LOGPIXELSX as i32) as f64;
        let dpi_y = GetDeviceCaps(hdc, LOGPIXELSY as i32) as f64;

        // GDI+ output submitted to a printer HDC (GdipDrawString etc.) gets
        // converted to vector paths by the driver using a fixed 96 DPI
        // assumption, REGARDLESS of the device's real resolution -- verified
        // against a real "Microsoft Print to PDF" DC reporting 600 DPI via
        // GetDeviceCaps (and via `GdipGetDpiY`, which also -- misleadingly
        // -- reports 600): the actual PDF page's coordinate transform
        // matrix corresponded to exactly 96 DPI, so GDI+-space pixels sized
        // using the device's true DPI rendered ~6x too large. Plain GDI
        // (CreateFontW/TextOutW) doesn't have this problem -- the driver
        // converts that to real PDF text operators with a correctly
        // device-DPI-aware font size. This is a known category of GDI+/printer
        // DPI mismatch, not specific to this one driver, so this fixed 96
        // baseline (rather than trusting either GetDeviceCaps or
        // GdipGetDpiY) is the correct fix, not a workaround for one test.
        const GDIPLUS_ASSUMED_DPI: f64 = 96.0;
        let gdi_to_gdiplus_px = (GDIPLUS_ASSUMED_DPI / dpi_y) as f32;
        let mm_to_px_x = |mm: f64| (mm / 25.4 * dpi_x).round() as i32;
        let mm_to_px_y = |mm: f64| (mm / 25.4 * dpi_y).round() as i32;
        let pt_to_px_y = |pt: f64| (pt / 72.0 * dpi_y).round() as i32;

        let margin_x = mm_to_px_x(margin_mm);
        let margin_y = mm_to_px_y(margin_mm);
        let page_width = mm_to_px_x(width_mm);
        let page_height = mm_to_px_y(height_mm);
        let content_width = page_width - margin_x * 2;
        let _ = page_height; // not currently needed beyond DC geometry, but kept for clarity/future pagination

        let create_font = |height_px: i32, bold: bool| -> HFONT {
            let face = to_wide("Arial");
            CreateFontW(
                -height_px,
                0,
                0,
                0,
                if bold { FW_BOLD as i32 } else { FW_NORMAL as i32 },
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32,
                DEFAULT_QUALITY as u32,
                DEFAULT_PITCH as u32, // pitch-and-family: default pitch, don't-care family (FF_DONTCARE == 0)
                face.as_ptr(),
            )
        };

        let measure_width = |text: &[u16]| -> i32 {
            let mut size = std::mem::zeroed::<windows_sys::Win32::Foundation::SIZE>();
            let len = text.len().saturating_sub(1) as i32; // exclude the NUL terminator
            GetTextExtentPoint32W(hdc, text.as_ptr(), len, &mut size);
            size.cx
        };

        let align_x = |align: Option<Align>, container_width: i32, content_width_px: i32| -> i32 {
            match align.unwrap_or_default() {
                Align::Center => (container_width - content_width_px) / 2,
                Align::Right => container_width - content_width_px,
                Align::Left => 0,
            }
        };

        let draw_filled_rect = |x: i32, y: i32, w: i32, h: i32| {
            PatBlt(hdc, x, y, w.max(1), h.max(1), BLACKNESS);
        };

        // Draws `text` via GDI+ (see the GdiplusStartup comment above for
        // why) using a font known to cover Myanmar ("Myanmar Text", built
        // into Windows 10/11), aligning it within (container_x,
        // container_width) like the plain-GDI text path does. A no-op if
        // GDI+ failed to initialize or the font/draw calls fail, rather than
        // aborting the whole print job over one unsupported line.
        let draw_myanmar_text = |text: &str, align: Option<Align>, container_x: f32, container_width: f32, y: f32, height_px: f32, bold: bool| {
            if gdiplus_graphics.is_null() {
                return;
            }
            // Convert every GDI-space coordinate/size to GDI+-space once, up
            // front, via the DPI-mismatch factor computed above -- from here
            // down, everything (measuring, aligning, drawing) stays entirely
            // in GDI+-space, so no further conversion is needed.
            let container_x = container_x * gdi_to_gdiplus_px;
            let container_width = container_width * gdi_to_gdiplus_px;
            let y = y * gdi_to_gdiplus_px;
            let height_px = height_px * gdi_to_gdiplus_px;

            let family_name = to_wide("Myanmar Text");
            let mut family: *mut GpFontFamily = null_mut();
            if GdipCreateFontFamilyFromName(family_name.as_ptr(), null_mut(), &mut family) != 0 {
                return;
            }
            let mut font: *mut GpFont = null_mut();
            let style = if bold { FontStyleBold } else { FontStyleRegular };
            if GdipCreateFont(family, height_px, style, UnitPixel, &mut font) != 0 {
                GdipDeleteFontFamily(family);
                return;
            }
            let mut brush: *mut GpSolidFill = null_mut();
            GdipCreateSolidFill(0xff000000, &mut brush); // opaque black, ARGB

            let wide_text = to_wide(text);
            let measure_rect = RectF { X: 0.0, Y: 0.0, Width: 10000.0, Height: height_px * 2.0 };
            let mut bounds: RectF = std::mem::zeroed();
            let mut chars_fitted = 0i32;
            let mut lines_filled = 0i32;
            GdipMeasureString(gdiplus_graphics, wide_text.as_ptr(), -1, font, &measure_rect, std::ptr::null(), &mut bounds, &mut chars_fitted, &mut lines_filled);

            let x = container_x
                + match align.unwrap_or_default() {
                    Align::Center => (container_width - bounds.Width) / 2.0,
                    Align::Right => container_width - bounds.Width,
                    Align::Left => 0.0,
                };
            let draw_rect = RectF { X: x, Y: y, Width: bounds.Width.max(1.0), Height: height_px * 2.0 };
            GdipDrawString(gdiplus_graphics, wide_text.as_ptr(), -1, font, &draw_rect, std::ptr::null(), brush as *const _);

            GdipDeleteBrush(brush as *mut _);
            GdipDeleteFont(font);
            GdipDeleteFontFamily(family);
        };

        let result: Result<(), AgentError> = (|| {
            let doc_name = to_wide(doc.meta.as_ref().and_then(|m| m.title.clone()).unwrap_or_else(|| "Print".to_string()).as_str());
            let doc_info = DOCINFOW { cbSize: std::mem::size_of::<DOCINFOW>() as i32, lpszDocName: doc_name.as_ptr(), lpszOutput: null_mut(), lpszDatatype: null_mut(), fwType: 0 };

            for _ in 0..copies.max(1) {
                if StartDocW(hdc, &doc_info) <= 0 {
                    return Err(AgentError::TransportError("StartDoc failed".to_string()));
                }
                if StartPage(hdc) <= 0 {
                    EndDoc(hdc);
                    return Err(AgentError::TransportError("StartPage failed".to_string()));
                }
                SetBkMode(hdc, TRANSPARENT as i32);

                let mut cursor_y = margin_y;
                for node in &doc.content {
                    match node {
                        PrintNode::Text(n) => {
                            let size_px = pt_to_px_y(FONT_SIZE_PT * n.size.map(|s| s.h as f64).unwrap_or(1.0));
                            if needs_shaping(&n.value) {
                                draw_myanmar_text(&n.value, n.align, margin_x as f32, content_width as f32, cursor_y as f32, size_px as f32, n.bold.unwrap_or(false));
                            } else {
                                let font = create_font(size_px, n.bold.unwrap_or(false));
                                let old_font = SelectObject(hdc, font as HGDIOBJ);
                                let wide_text = to_wide(&n.value);
                                let text_width = measure_width(&wide_text);
                                let x = margin_x + align_x(n.align, content_width, text_width);
                                TextOutW(hdc, x, cursor_y, wide_text.as_ptr(), (wide_text.len() - 1) as i32);
                                SelectObject(hdc, old_font);
                                DeleteObject(font as HGDIOBJ);
                            }
                            cursor_y += (size_px as f64 * LINE_HEIGHT_FACTOR).round() as i32;
                        }
                        PrintNode::Row(n) => {
                            let size_px = pt_to_px_y(FONT_SIZE_PT);
                            let bold = n.bold.unwrap_or(false);
                            let font = create_font(size_px, bold);
                            let old_font = SelectObject(hdc, font as HGDIOBJ);
                            let total_units: f64 = n.columns.iter().map(|c| c.width).sum();
                            let mut x_cursor = margin_x;
                            for col in &n.columns {
                                let col_width = ((col.width / total_units) * content_width as f64).round() as i32;
                                if needs_shaping(&col.text) {
                                    draw_myanmar_text(&col.text, col.align, x_cursor as f32, col_width as f32, cursor_y as f32, size_px as f32, bold);
                                } else {
                                    let wide_text = to_wide(&col.text);
                                    let text_width = measure_width(&wide_text);
                                    let x = x_cursor + align_x(col.align, col_width, text_width);
                                    TextOutW(hdc, x, cursor_y, wide_text.as_ptr(), (wide_text.len() - 1) as i32);
                                }
                                x_cursor += col_width;
                            }
                            SelectObject(hdc, old_font);
                            DeleteObject(font as HGDIOBJ);
                            cursor_y += (size_px as f64 * LINE_HEIGHT_FACTOR).round() as i32;
                        }
                        PrintNode::Rule(_) => {
                            let y = cursor_y + pt_to_px_y(2.0);
                            let mut pt = std::mem::zeroed();
                            MoveToEx(hdc, margin_x, y, &mut pt);
                            LineTo(hdc, margin_x + content_width, y);
                            cursor_y += pt_to_px_y(FONT_SIZE_PT * LINE_HEIGHT_FACTOR / 2.0);
                        }
                        PrintNode::Feed(n) => {
                            cursor_y += pt_to_px_y(FONT_SIZE_PT * LINE_HEIGHT_FACTOR) * n.lines.unwrap_or(1) as i32;
                        }
                        PrintNode::Image(n) => {
                            match draw_image(hdc, n, margin_x, cursor_y, content_width) {
                                Ok(height_px) => cursor_y += height_px,
                                Err(e) => return Err(e),
                            }
                        }
                        PrintNode::Barcode(n) => {
                            let bar_height = pt_to_px_y(60.0);
                            match generate_barcode_bits(n.symbology, &n.data) {
                                None => {
                                    let label = format!("[{}: {}]", n.symbology.name().to_uppercase(), n.data);
                                    let font = create_font(pt_to_px_y(FONT_SIZE_PT), false);
                                    let old_font = SelectObject(hdc, font as HGDIOBJ);
                                    let wide = to_wide(&label);
                                    TextOutW(hdc, margin_x, cursor_y, wide.as_ptr(), (wide.len() - 1) as i32);
                                    SelectObject(hdc, old_font);
                                    DeleteObject(font as HGDIOBJ);
                                    cursor_y += pt_to_px_y(FONT_SIZE_PT * LINE_HEIGHT_FACTOR);
                                }
                                Some(bits) => {
                                    let module_width = (content_width as f64 / bits.len() as f64).max(1.0);
                                    for (i, b) in bits.bytes().enumerate() {
                                        if b == b'1' {
                                            draw_filled_rect(margin_x + (i as f64 * module_width) as i32, cursor_y, module_width.ceil() as i32, bar_height);
                                        }
                                    }
                                    cursor_y += bar_height;
                                    if n.show_text != Some(false) {
                                        let font = create_font(pt_to_px_y(FONT_SIZE_PT), false);
                                        let old_font = SelectObject(hdc, font as HGDIOBJ);
                                        let wide = to_wide(&n.data);
                                        let text_width = measure_width(&wide);
                                        let x = margin_x + (content_width - text_width) / 2;
                                        TextOutW(hdc, x, cursor_y, wide.as_ptr(), (wide.len() - 1) as i32);
                                        SelectObject(hdc, old_font);
                                        DeleteObject(font as HGDIOBJ);
                                        cursor_y += pt_to_px_y(FONT_SIZE_PT * LINE_HEIGHT_FACTOR);
                                    }
                                }
                            }
                        }
                        PrintNode::QrCode(n) => match qr_modules(&n.data, n.ec.unwrap_or(QrEcLevel::M)) {
                            Ok(qr) => {
                                let px = content_width.min(pt_to_px_y(FONT_SIZE_PT * 6.0));
                                let module_size = (px as f64 / qr.size as f64).max(1.0);
                                let offset_x = margin_x + (content_width - px) / 2;
                                for row in 0..qr.size {
                                    for col in 0..qr.size {
                                        if qr.is_dark(row, col) {
                                            draw_filled_rect(
                                                offset_x + (col as f64 * module_size) as i32,
                                                cursor_y + (row as f64 * module_size) as i32,
                                                module_size.ceil() as i32,
                                                module_size.ceil() as i32,
                                            );
                                        }
                                    }
                                }
                                cursor_y += px;
                            }
                            Err(e) => return Err(AgentError::TransportError(e.to_string())),
                        },
                        PrintNode::Cut(_) | PrintNode::Drawer(_) | PrintNode::Raw(_) => {} // hardware-only actions; no-op for GDI
                    }
                }

                EndPage(hdc);
                EndDoc(hdc);
            }
            Ok(())
        })();

        if !gdiplus_graphics.is_null() {
            GdipDeleteGraphics(gdiplus_graphics);
        }
        GdiplusShutdown(gdiplus_token);
        DeleteDC(hdc);
        result
    }
}

/// Decodes `node.data` (base64 PNG) and blits it via `StretchDIBits`. Returns
/// the drawn height in device pixels so the caller can advance its cursor.
#[cfg(target_os = "windows")]
unsafe fn draw_image(hdc: windows_sys::Win32::Graphics::Gdi::HDC, node: &printers_core::types::ImageNode, x: i32, y: i32, content_width: i32) -> Result<i32, AgentError> {
    use windows_sys::Win32::Graphics::Gdi::*;

    use base64::Engine;
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(&node.data)
        .map_err(|e| AgentError::TransportError(format!("invalid base64: {e}")))?;
    let img = image::load_from_memory(&png_bytes).map_err(|e| AgentError::TransportError(format!("invalid image: {e}")))?.to_rgb8();
    let (src_width, src_height) = (img.width() as i32, img.height() as i32);
    let dest_width = node.width_px.map(|w| w as i32).unwrap_or(src_width).min(content_width).max(1);
    let dest_height = ((dest_width as f64 / src_width as f64) * src_height as f64).round() as i32;

    // DIBs are bottom-up by convention (positive biHeight) with rows padded
    // to a 4-byte boundary; build that layout from the top-down RGB buffer
    // `image` gives us.
    let row_stride = ((src_width * 3 + 3) / 4) * 4;
    let mut dib = vec![0u8; (row_stride * src_height) as usize];
    for (px, py, pixel) in img.enumerate_pixels() {
        let dst_row = src_height as u32 - 1 - py; // flip to bottom-up
        let offset = (dst_row as i32 * row_stride + px as i32 * 3) as usize;
        dib[offset] = pixel.0[2]; // B
        dib[offset + 1] = pixel.0[1]; // G
        dib[offset + 2] = pixel.0[0]; // R
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: src_width,
            biHeight: src_height,
            biPlanes: 1,
            biBitCount: 24,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [windows_sys::Win32::Graphics::Gdi::RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }],
    };

    StretchDIBits(hdc, x, y, dest_width, dest_height, 0, 0, src_width, src_height, dib.as_ptr() as *const _, &bmi, DIB_RGB_COLORS, SRCCOPY);

    Ok(dest_height)
}
