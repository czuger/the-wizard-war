// Map interactions: site menus, and moves staged on the map then sent with the phase form.
// A mover ([data-mover]) lists the [data-target] values it may reach in data-targets; staging
// a move adds hidden inputs to #phase-form (mover=target, plus the cargo field for pawns
// leaving the city) and draws an arrow.
(function () {
  const board = document.querySelector('.board');
  const form = document.getElementById('phase-form');
  if (!board) return;
  const SVG_NS = 'http://www.w3.org/2000/svg';
  const BOARD_WIDTH = 2780;
  let boardScale = 1;
  let freeLeft = 0;
  let freeRight = window.innerWidth;
  const arrows = board.querySelector('.board-moves');
  const movers = Array.from(board.querySelectorAll('[data-mover]'));
  const counter = document.querySelector('[data-move-counter]');
  const staged = new Map();
  let selected = null;
  let chooser = null;

  function closeMenus(except) {
    board.querySelectorAll('details[open]').forEach(function (menu) {
      if (menu !== except) menu.open = false;
    });
  }

  board.querySelectorAll('details').forEach(function (menu) {
    menu.addEventListener('toggle', function () {
      if (!menu.open) return;
      closeMenus(menu);
      const content = menu.querySelector(':scope > .menu');
      if (content) keepInWindow(content);
    });
  });

  // The board keeps its pixel coordinates and is scaled as a whole to the width left between the open
  // floating panels, so that none of them hides a site; the map scrolls vertically.
  function fitBoard() {
    const gap = 8;
    const viewport = board.closest('.board-viewport');
    const viewportWidth = viewport.clientWidth;
    let left = 0;
    let right = viewportWidth;
    document.querySelectorAll('.floating[open]').forEach(function (panel) {
      const rect = panel.getBoundingClientRect();
      if (rect.left < viewportWidth / 2) left = Math.max(left, rect.right + gap);
      else right = Math.min(right, rect.left - gap);
    });
    const width = Math.max(right - left, 0);
    const scale = width / BOARD_WIDTH;
    viewport.style.setProperty('--board-scale', scale);
    viewport.style.setProperty('--board-center', left + width / 2 + 'px');
    boardScale = scale;
    freeLeft = left;
    freeRight = right;
  }

  // Menus open below their site; flip them up or shift them sideways when they would leave the window or
  // slide under a floating panel.
  function keepInWindow(menu) {
    const margin = 8;
    menu.classList.remove('menu-up');
    menu.style.marginLeft = '';
    let rect = menu.getBoundingClientRect();
    const anchor = menu.parentElement.getBoundingClientRect();
    const roomBelow = window.innerHeight - anchor.bottom;
    if (rect.bottom > window.innerHeight - margin && anchor.top > roomBelow) menu.classList.add('menu-up');
    rect = menu.getBoundingClientRect();
    let shift = 0;
    if (rect.left < freeLeft + margin) shift = freeLeft + margin - rect.left;
    else if (rect.right > freeRight - margin) shift = freeRight - margin - rect.right;
    if (shift) menu.style.marginLeft = shift / boardScale + 'px';
  }

  // The page reloads on every game change: keep the floating panels as the player left them.
  function rememberPanels() {
    document.querySelectorAll('details[data-remember]').forEach(function (panel) {
      const key = 'panel-' + panel.dataset.remember;
      try {
        const saved = localStorage.getItem(key);
        if (saved !== null) panel.open = saved === 'open';
      } catch (error) {
        // Storage unavailable: panels keep their default state.
      }
      panel.addEventListener('toggle', function () {
        fitBoard();
        try {
          localStorage.setItem(key, panel.open ? 'open' : 'closed');
        } catch (error) {
          // Storage unavailable: the state is simply not remembered.
        }
      });
    });
  }

  document.addEventListener('keydown', function (event) {
    if (event.key !== 'Escape') return;
    closeMenus(null);
    closeChooser();
    select(null);
  });

  function targetElement(value) {
    return board.querySelector('[data-target="' + value + '"]');
  }

  function point(value) {
    const element = targetElement(value);
    return element && { x: Number(element.dataset.x), y: Number(element.dataset.y) };
  }

  function select(mover) {
    if (selected) selected.classList.remove('is-selected');
    board.querySelectorAll('.is-target').forEach(function (element) {
      element.classList.remove('is-target');
    });
    selected = mover;
    if (!mover) return;
    mover.classList.add('is-selected');
    mover.dataset.targets.split(' ').forEach(function (value) {
      const element = targetElement(value);
      if (element) element.classList.add('is-target');
    });
  }

  function cargoLeft(item) {
    const palette = document.querySelector('#cargo-palette [data-item="' + item + '"]');
    let left = palette ? Number(palette.dataset.count) : 0;
    staged.forEach(function (move) {
      if (move.item === item) left -= 1;
    });
    return left;
  }

  function closeChooser() {
    if (chooser) chooser.remove();
    chooser = null;
  }

  // A pawn leaving the city loads one stock item: let the player pick it next to the gate.
  function chooseCargo(mover, target) {
    closeChooser();
    const at = point(target);
    chooser = document.createElement('div');
    chooser.className = 'menu cargo-chooser';
    chooser.style.left = at.x + 'px';
    chooser.style.top = at.y + 'px';
    const title = document.createElement('p');
    title.className = 'menu-title';
    title.textContent = 'Quel objet emporter ?';
    chooser.appendChild(title);
    document.querySelectorAll('#cargo-palette [data-item]').forEach(function (palette) {
      const item = palette.dataset.item;
      const left = cargoLeft(item);
      const button = document.createElement('button');
      button.type = 'button';
      button.disabled = left <= 0;
      const image = document.createElement('img');
      image.src = palette.dataset.image;
      image.alt = '';
      image.className = 'item-icon';
      button.append(image, ' ' + palette.dataset.label + ' (' + left + ' en stock)');
      button.addEventListener('click', function (event) {
        event.stopPropagation();
        closeChooser();
        stage(mover, target, item);
      });
      chooser.appendChild(button);
    });
    if (chooser.querySelectorAll('button:not([disabled])').length === 0) {
      const empty = document.createElement('p');
      empty.textContent = 'Votre stock est vide : ce pion ne peut pas sortir.';
      chooser.appendChild(empty);
    }
    board.appendChild(chooser);
    keepInWindow(chooser);
  }

  function hiddenInput(name, value) {
    const input = document.createElement('input');
    input.type = 'hidden';
    input.name = name;
    input.value = value;
    form.appendChild(input);
    return input;
  }

  function drawArrow(from, to) {
    const start = point(from);
    const end = point(to);
    if (!start || !end || !arrows) return null;
    const line = document.createElementNS(SVG_NS, 'line');
    line.setAttribute('x1', start.x);
    line.setAttribute('y1', start.y);
    line.setAttribute('x2', end.x);
    line.setAttribute('y2', end.y);
    line.setAttribute('marker-end', 'url(#move-arrow)');
    arrows.appendChild(line);
    return line;
  }

  function stage(mover, target, item) {
    const inputs = [hiddenInput(mover.dataset.mover, target)];
    if (item) inputs.push(hiddenInput(mover.dataset.cargo, item));
    const arrow = drawArrow(mover.dataset.from, target);
    staged.set(mover, { inputs: inputs, arrow: arrow, item: item });
    mover.classList.add('is-staged');
    select(null);
    update();
  }

  function unstage(mover) {
    const move = staged.get(mover);
    if (!move) return;
    move.inputs.forEach(function (input) { input.remove(); });
    if (move.arrow) move.arrow.remove();
    staged.delete(mover);
    mover.classList.remove('is-staged');
    update();
  }

  function update() {
    if (counter) counter.textContent = staged.size + ' / ' + movers.length + ' pion(s) déplacé(s)';
    if (form && form.hasAttribute('data-require-all')) {
      const submit = form.querySelector('button[type="submit"]');
      if (submit) submit.disabled = staged.size < movers.length;
    }
  }

  function chooseTarget(target) {
    const value = target.dataset.target;
    if (selected.dataset.cargo) {
      chooseCargo(selected, value);
    } else {
      stage(selected, value, null);
    }
  }

  movers.forEach(function (mover) {
    mover.addEventListener('click', function (event) {
      event.stopPropagation();
      const host = mover.closest('.is-target');
      if (selected && selected !== mover && host) {
        chooseTarget(host);
        return;
      }
      closeChooser();
      if (staged.has(mover)) {
        unstage(mover);
        select(mover);
      } else {
        select(selected === mover ? null : mover);
      }
    });
  });

  board.addEventListener('click', function (event) {
    if (!selected || (chooser && chooser.contains(event.target))) return;
    const target = event.target.closest('.is-target');
    if (!target) {
      closeChooser();
      return;
    }
    chooseTarget(target);
  });

  const reset = document.querySelector('[data-reset-moves]');
  if (reset) {
    reset.addEventListener('click', function () {
      Array.from(staged.keys()).forEach(unstage);
      closeChooser();
      select(null);
    });
  }

  update();

  rememberPanels();
  fitBoard();
  window.addEventListener('resize', fitBoard);
  board.querySelectorAll('details[open] > .menu').forEach(keepInWindow);
})();
