// Map interactions: site menus, and moves staged on the map then sent with the phase form.
// A mover ([data-mover]) lists the [data-target] values it may reach in data-targets; staging
// a move adds hidden inputs to #phase-form (mover=target, plus the cargo field for pawns
// leaving the city) and draws an arrow.
(function () {
  const board = document.querySelector('.board');
  const form = document.getElementById('phase-form');
  if (!board) return;
  const SVG_NS = 'http://www.w3.org/2000/svg';
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
      if (menu.open) closeMenus(menu);
    });
  });

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

  // Bring the place to act on into view, since the map is larger than its viewport.
  const focus = board.querySelector('.site-menu[open], .mover, .site-menu, .token-menu');
  const viewport = board.parentElement;
  if (focus && viewport) {
    const view = viewport.getBoundingClientRect();
    const place = focus.getBoundingClientRect();
    viewport.scrollLeft += place.left - view.left - (view.width - place.width) / 2;
    viewport.scrollTop += place.top - view.top - (view.height - place.height) / 2;
  }
})();
