// The remote login browser: draws the server's screen onto a canvas and
// sends mouse and keyboard input back over a WebSocket.
//
// Where the browser supports HTML-in-Canvas (an experimental API, see
// https://github.com/WICG/html-in-canvas), the page's sign-in fields are
// mirrored as real inputs drawn into the canvas, where password managers
// can find and fill them. Other browsers get the same screen without the
// mirrors.

const canvas = document.getElementById("screen");
const context = canvas.getContext("2d");
const status = document.getElementById("login-status");
const done = document.getElementById("done");
const hint = document.getElementById("autofill-hint");
const host = document.getElementById("login-host");

const scheme = location.protocol === "https:" ? "wss:" : "ws:";
let socket = null;

function send(input) {
  if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify(input));
}

// ---- the screen --------------------------------------------------------------

let frame = null;
let received = 0;
let shown = 0;

function draw() {
  if (mirrors) {
    canvas.requestPaint();
  } else if (frame) {
    context.drawImage(frame, 0, 0, canvas.width, canvas.height);
  }
}

async function receive(event) {
  if (typeof event.data === "string") {
    const message = JSON.parse(event.data);
    if (message.type === "size") {
      // The page's size in CSS pixels; the canvas takes it on, so canvas
      // pixels are the coordinates input is replayed at.
      canvas.width = message.width;
      canvas.height = message.height;
      mirrors?.layout();
      draw();
    } else if (message.type === "fields") {
      mirrors?.update(message.fields);
    }
    return;
  }
  // Decoding is asynchronous; never let an older frame replace a newer one.
  const sequence = ++received;
  const bitmap = await createImageBitmap(event.data);
  if (sequence < shown) {
    bitmap.close();
    return;
  }
  shown = sequence;
  frame?.close();
  frame = bitmap;
  draw();
}

function connect() {
  socket = new WebSocket(`${scheme}//${location.host}/settings/sources/login/ws`);
  socket.binaryType = "blob";
  socket.addEventListener("message", receive);
  // A dropped connection (or a server restart) is picked up again while
  // the login browser is still open; the status line says when it is not.
  socket.addEventListener("close", () => {
    setTimeout(async () => {
      if ((await state())?.open) connect();
    }, 1000);
  });
}

// ---- input -------------------------------------------------------------------

// Canvas pixels map one to one onto the remote page, whatever size the
// canvas is shown at.
function position(event) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: ((event.clientX - rect.left - canvas.clientLeft) * canvas.width) / canvas.clientWidth,
    y: ((event.clientY - rect.top - canvas.clientTop) * canvas.height) / canvas.clientHeight,
  };
}

// Events aimed at a mirrored field belong to it, not to the remote screen.
const onScreen = (event) => event.target === canvas;

let lastMove = 0;
canvas.addEventListener("mousemove", (event) => {
  const now = performance.now();
  if (!onScreen(event) || now - lastMove < 40) return;
  lastMove = now;
  send({ type: "mouse_move", ...position(event) });
});
canvas.addEventListener("mousedown", (event) => {
  if (!onScreen(event)) return;
  canvas.focus();
  send({ type: "mouse_down", ...position(event), button: event.button });
  event.preventDefault();
});
canvas.addEventListener("mouseup", (event) => {
  if (!onScreen(event)) return;
  send({ type: "mouse_up", ...position(event), button: event.button });
});
canvas.addEventListener(
  "wheel",
  (event) => {
    send({ type: "wheel", ...position(event), dx: event.deltaX, dy: event.deltaY });
    event.preventDefault();
  },
  { passive: false },
);
canvas.addEventListener("contextmenu", (event) => {
  if (onScreen(event)) event.preventDefault();
});

// CDP's modifier bits.
function modifiers(event) {
  return (
    (event.altKey ? 1 : 0) |
    (event.ctrlKey ? 2 : 0) |
    (event.metaKey ? 4 : 0) |
    (event.shiftKey ? 8 : 0)
  );
}

canvas.addEventListener("keydown", (event) => {
  if (!onScreen(event)) return;
  // Let the paste event deliver clipboard text.
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "v") return;
  send({ type: "key_down", key: event.key, code: event.code, modifiers: modifiers(event) });
  event.preventDefault();
});
canvas.addEventListener("keyup", (event) => {
  if (!onScreen(event)) return;
  send({ type: "key_up", key: event.key, code: event.code, modifiers: modifiers(event) });
  event.preventDefault();
});
canvas.addEventListener("paste", (event) => {
  if (!onScreen(event)) return;
  const text = event.clipboardData?.getData("text");
  if (text) send({ type: "text", text });
  event.preventDefault();
});

// ---- mirrored sign-in fields (HTML-in-Canvas only) ----------------------------

// How each kind of field presents itself to password managers.
const KINDS = {
  username: { type: "text", autocomplete: "username", name: "username", inputMode: "email" },
  password: { type: "password", autocomplete: "current-password", name: "password" },
  one_time_code: {
    type: "text",
    autocomplete: "one-time-code",
    name: "one-time-code",
    inputMode: "numeric",
  },
};

// Each remote field gets a real input, drawn into the canvas over the
// field's picture. The inputs are transparent, so what shows is always the
// remote page; what is typed or filled into them is sent to the page.
function createMirrors() {
  // Opts the canvas's children into layout and drawing, under the
  // attribute names of both revisions of the proposal. Only direct
  // children are hit tested, so a form elsewhere owns the inputs.
  canvas.setAttribute("layoutsubtree", "");
  canvas.setAttribute("content", "drawable");
  const form = document.getElementById("mirror");
  /** key → { input, field, timer } */
  const entries = new Map();

  function fill(entry) {
    clearTimeout(entry.timer);
    entry.timer = null;
    send({ type: "fill", key: entry.field.key, value: entry.input.value });
  }

  // Typing bursts become one fill.
  function schedule(entry) {
    clearTimeout(entry.timer);
    entry.timer = setTimeout(() => fill(entry), 60);
  }

  // Enter, or a password manager submitting: send what is pending, then
  // press Enter in the page.
  function submit(entry) {
    for (const other of entries.values()) if (other.timer) fill(other);
    send({ type: "focus", key: entry.field.key });
    send({ type: "key_down", key: "Enter", code: "Enter", modifiers: 0 });
    send({ type: "key_up", key: "Enter", code: "Enter", modifiers: 0 });
  }

  function create(field) {
    const kind = KINDS[field.kind] ?? KINDS.username;
    const input = document.createElement("input");
    input.setAttribute("drawable", "");
    input.setAttribute("form", form.id);
    input.type = kind.type;
    input.autocomplete = kind.autocomplete;
    input.name = kind.name;
    if (kind.inputMode) input.inputMode = kind.inputMode;
    input.spellcheck = false;
    input.autocapitalize = "off";
    Object.assign(input.style, {
      display: "block",
      boxSizing: "border-box",
      margin: "0",
      padding: "0",
      border: "0",
      outline: "none",
      background: "transparent",
      color: "transparent",
      webkitTextFillColor: "transparent",
      caretColor: "transparent",
      // Keeps the browser's own autofill tint from covering the field.
      transition: "background-color 1000000s",
    });
    const entry = { input, field, timer: null };
    input.addEventListener("input", () => schedule(entry));
    input.addEventListener("change", () => fill(entry));
    input.addEventListener("focus", () => send({ type: "focus", key: entry.field.key }));
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        submit(entry);
      }
    });
    canvas.append(input);
    return entry;
  }

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const all = [...entries.values()];
    const entry = all.find(({ input }) => input === document.activeElement) ?? all.at(-1);
    if (entry) submit(entry);
  });

  // An input's CSS size must match its drawn size on screen, which depends
  // on how large the canvas is shown.
  function layout() {
    const scale = canvas.clientWidth / canvas.width;
    for (const { input, field } of entries.values()) {
      input.style.width = `${field.width * scale}px`;
      input.style.height = `${field.height * scale}px`;
    }
  }

  function update(fields) {
    const keys = new Set(fields.map((field) => field.key));
    for (const [key, entry] of entries) {
      if (!keys.has(key)) {
        clearTimeout(entry.timer);
        entry.input.remove();
        entries.delete(key);
      }
    }
    for (const field of fields) {
      let entry = entries.get(field.key);
      if (entry) entry.field = field;
      else entries.set(field.key, (entry = create(field)));
      entry.input.placeholder = field.label;
      entry.input.setAttribute("aria-label", field.label);
      // Show what the page holds, unless the admin is busy in this field.
      if (field.value !== null && !entry.timer && document.activeElement !== entry.input) {
        entry.input.value = field.value;
      }
    }
    layout();
    canvas.requestPaint();
  }

  canvas.addEventListener("paint", () => {
    context.reset();
    if (frame) context.drawImage(frame, 0, 0, canvas.width, canvas.height);
    for (const { input, field } of entries.values()) {
      try {
        const transform = context.drawElementImage(
          input,
          field.x,
          field.y,
          field.width,
          field.height,
        );
        // The first revision of the proposal leaves moving the element to
        // where it was drawn (for hit testing, and for password managers
        // measuring it) to the page; later ones do it themselves and
        // return nothing.
        if (transform) {
          const css = transform.toString();
          if (input.style.transform !== css) input.style.transform = css;
        }
      } catch {
        // No snapshot of a new input yet; the next paint draws it.
      }
    }
  });

  new ResizeObserver(() => {
    layout();
    canvas.requestPaint();
  }).observe(canvas);

  return { update, layout };
}

// Both revisions of the proposal so far have these two.
const htmlInCanvas =
  typeof context.drawElementImage === "function" && typeof canvas.requestPaint === "function";
const mirrors = htmlInCanvas ? createMirrors() : null;

hint.closest("[data-hint]").dataset.mirrors = String(Boolean(mirrors));
hint.textContent = mirrors
  ? `Your password manager can fill the sign-in fields. It sees them on ${location.origin}, so add that address to your Google login.`
  : "Want your password manager to fill this in? In Chrome, enable chrome://flags/#canvas-draw-element and reload. The sign-in fields then get real inputs laid over them. Typing works fine without it.";

// ---- status --------------------------------------------------------------------

async function state() {
  try {
    return await (await fetch("/settings/sources/login/status")).json();
  } catch {
    return null;
  }
}

// Enable "Done" once the browser holds a login.
async function poll() {
  const current = await state();
  if (!current) return;
  done.disabled = !current.logged_in;
  status.closest("[data-login-pill]").dataset.signedIn = String(current.logged_in);
  status.textContent = current.logged_in
    ? "Signed in: press Done"
    : current.open
      ? "Not signed in yet"
      : "The login browser closed";
  host.textContent = current.host ?? (current.open ? "loading…" : "closed");
}
setInterval(poll, 2000);
poll();
connect();
canvas.focus();
