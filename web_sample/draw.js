import {
  new_frame,
  set_text,
  event_mouse_pos_clear,
  scene_from_json,
  scene_to_json,
  event_scroll,
  ProjectionMode,
  set_bg_color,
  set_dither_count_frames,
  event_mouse_pos,
  event_mouse_button_state,
  event_focus,
  new_scene,
  add_cube,
  render,
  set_camera_aspect,
  add_line,
  set_obj_transform,
  color_conv,
  TermColorMode,
  StandardMaterial,
  vec3,
  default as init
} from './modules/asciirend.js';

function uiVec3(x, y, z, w, h) {
  return vec3(100 / w * x, z, 100 / h * y)
}

function updateUiText(scene, obj, text, x, y, w, h) {
  const menu_text = set_text(scene, obj, text);
  if (text != null) {
    set_obj_transform(scene, obj, uiVec3(Math.floor(x + (text.length + 2) / 2), y, 2, w, h), vec3(0, 0, 0), uiVec3(text.length + 2, 3, 1, w, h));
  } else {
    set_obj_transform(scene, obj, uiVec3(0, 0, 0), vec3(0, 0, 0), uiVec3(0, 0, 0));
  }
  return menu_text
}

function uiText(scene, text, x, y, w, h) {
  const menu_text = add_cube(scene, StandardMaterial.UiText, vec3(1, 1, 1), text);
  if (text != null) {
    set_obj_transform(scene, menu_text, uiVec3(Math.floor(x + (text.length + 2) / 2), y, 2, w, h), vec3(0, 0, 0), uiVec3(text.length + 2, 3, 1, w, h));
  }
  return menu_text
}

function charWidth(divElement) {
  // Currently we hardcode the character ratios. Using canvas, we could measure them exactly, but
  // I am not good enough for this.
  /*
  // Create a canvas element to measure text width
  const canvas = document.createElement('canvas');
  const context = canvas.getContext('2d');
  */

  const computedStyle = window.getComputedStyle(divElement);
  const fontFamily = computedStyle.getPropertyValue('font-family');
  const fontSize = computedStyle.getPropertyValue('font-size');

  /*
  // Set the font size and family to match the ASCII art
  context.font = `${fontSize}px ${fontFamily}`;

  // Measure the width of a single character
  const measurement = context.measureText('#');
  console.log(measurement)
  const actualSize = measurement.fontBoundingBoxAscent + measurement.fontBoundingBoxDescent;
  */
  const parsedSize = parseInt(fontSize, 10);

  return [parsedSize / 2.04, parsedSize]
}

export default async function ascii_render(id, scene_json, is_ortho, fov, znear, zfar, showUsage) {
  await init();

  const conv = color_conv(TermColorMode.SingleCol);

  const scene = scene_from_json(scene_json);

  const dom = document.getElementById(id);
  const [cw, ch] = charWidth(dom);

  let w = Math.floor(dom.clientWidth / cw);
  let h = Math.floor(dom.clientHeight / ch);

  console.log(w, h, cw, ch);

  const usage = showUsage ? uiText(scene, "Click and drag", 0, h - 1, w, h) : null;

  const startTime = performance.now();

  function mousePos(e) {
    if (e.touches != null) {
      e.preventDefault();
      e = e.touches[0];
    }
    const rect = e.target.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;

    event_mouse_pos(scene, x * w, y * h);
  }

  function mouseButton(e, down) {
    if (down) {
      mousePos(e);
    } else {
      event_mouse_pos_clear(scene);
    }

    let isRightMB = false;

    if ("which" in e) // Gecko (Firefox), WebKit (Safari/Chrome) & Opera
      isRightMB = e.which == 3;
    else if ("button" in e) // IE, Opera
      isRightMB = e.button == 2;

    event_mouse_button_state(scene, down, !isRightMB);
  }

  let focused = false;

  function focusEnter() {
    focused = true;
    event_focus(scene, true);
    if (usage) {
      updateUiText(scene, usage, null, 0, h - 1, w, h);
    }
  }

  function focusExit() {
    focused = false;
    event_mouse_button_state(scene, false, true);
    event_mouse_button_state(scene, false, false);
    event_focus(scene, false)
    if (usage) {
      updateUiText(scene, usage, "Click and drag", 0, h - 1, w, h);
    }
  }

  function scrollEvent(e) {
    if (focused) {
      event_scroll(scene, -e.deltaX, -e.deltaY);
      e.preventDefault();
    }
  }

  let prevDist = null;

  function touchMove(e) {
    if (e.touches.length == 1) {
      e.preventDefault();
      mousePos(e.touches[0]);
      prevDist = null;
    } else if (e.touches.length == 2) {
      e.preventDefault();
      const distX = e.touches[0].clientX - e.touches[1].clientX;
      const distY = e.touches[0].clientY - e.touches[1].clientY;
      const dist = Math.sqrt(distX * distX + distY * distY);

      if (prevDist !== null) {
        const diff = dist - prevDist;
        event_scroll(scene, 0, diff);
      }

      prevDist = dist;
    } else {
      prevDist = null;
    }
  }

  addEventListener("wheel", scrollEvent, {
    passive: false
  });
  dom.onclick = mousePos;
  dom.onmousemove = mousePos;
  dom.onmousedown = (e) => mouseButton(e, true);
  dom.onmouseup = (e) => mouseButton(e, false);
  dom.onmouseout = (e) => focusExit();
  dom.onmouseover = (e) => focusEnter();

  dom.addEventListener("touchmove", touchMove);
  dom.addEventListener("touchstart", (e) => {
    mouseButton(e, true);
    focusEnter();
  });
  dom.addEventListener("touchend", (e) => {
    mouseButton(e, false);
    focusExit();
    prevDist = null;
  });

  function nf() {
    set_camera_aspect(scene, (w / 2) / h);
    new_frame(scene, w, h);
  }

  nf()

  const v = setInterval(function() {
    w = Math.floor(dom.clientWidth / cw);
    h = Math.floor(dom.clientHeight / ch);

    const elapsed = performance.now() - startTime;
    performance.mark("asciirend-start-frame");

    const render_res = render(scene, conv, w, h, elapsed / 1000);
    performance.mark("asciirend-rendered-frame");

    let lines = '<pre style="pointer-events: none;">';

    for (let y = 0; y < h; y++) {
      // Can't really do color, because we'd end up with far too many dom elements.
      // Instead, just render grayscale.
      const bytes = render_res.slice(y * w, y * w + w).map(v => v.c);
      const line = String.fromCharCode(...bytes);
      lines += line;
      lines += "<br/>";
    }
    performance.mark("asciirend-converted-frame");

    const elapsed2 = performance.now() - startTime;

    dom.innerHTML = lines + "</pre>";

    for (let i = 0; i < render_res.length; i++) {
      // Make sure you free the pixels, or else you'll find a nasty surprise!
      render_res[i].free();
    }

    performance.mark("asciirend-drawn-frame");
    //console.log(elapsed, elapsed2 - elapsed, performance.measure("frame-time", "asciirend-start-frame", "asciirend-converted-frame"), render_res.length);

    nf()
  }, 4);
}
