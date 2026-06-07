/* ============================================================
   RazerClean — landing page interactions
   ============================================================ */
(function () {
  'use strict';

  // Current year in footer
  var yearEl = document.getElementById('year');
  if (yearEl) yearEl.textContent = new Date().getFullYear();

  // Mobile nav toggle
  var toggle = document.getElementById('navToggle');
  var nav = document.getElementById('siteNav');
  if (toggle && nav) {
    toggle.addEventListener('click', function () {
      var open = nav.classList.toggle('open');
      toggle.classList.toggle('open', open);
      toggle.setAttribute('aria-expanded', open ? 'true' : 'false');
    });
    nav.addEventListener('click', function (e) {
      if (e.target.tagName === 'A') {
        nav.classList.remove('open');
        toggle.classList.remove('open');
        toggle.setAttribute('aria-expanded', 'false');
      }
    });
  }

  // Quote form -> WhatsApp / Email
  var form = document.getElementById('quoteForm');
  var WHATSAPP_NUMBER = '27823093709';
  var EMAIL = 'razercleaning@gmail.com';

  function validate() {
    var ok = true;
    ['name', 'phone', 'location', 'service'].forEach(function (n) {
      var el = form.elements[n];
      if (el && !el.value.trim()) { el.classList.add('field-error'); ok = false; }
      else if (el) { el.classList.remove('field-error'); }
    });
    return ok;
  }

  function buildMessage() {
    var f = form.elements;
    var lines = [
      'New cleaning quote request — RazerClean',
      '',
      'Name: ' + (f.name.value || '-'),
      'Contact: ' + (f.phone.value || '-'),
      'Location: ' + (f.location.value || '-'),
      'Service: ' + (f.service.value || '-'),
      'Items / rooms: ' + (f.items.value || '-'),
      'Preferred date: ' + (f.date.value || '-'),
      'Notes: ' + (f.notes.value || '-')
    ];
    return lines.join('\n');
  }

  if (form) {
    form.addEventListener('submit', function (e) {
      e.preventDefault();
      if (!validate()) return;
      var url = 'https://wa.me/' + WHATSAPP_NUMBER + '?text=' + encodeURIComponent(buildMessage());
      window.open(url, '_blank', 'noopener');
    });

    var emailBtn = document.getElementById('emailBtn');
    if (emailBtn) {
      emailBtn.addEventListener('click', function () {
        if (!validate()) return;
        var subject = 'Cleaning Quote Request — ' + (form.elements.service.value || 'RazerClean');
        var url = 'mailto:' + EMAIL + '?subject=' + encodeURIComponent(subject) +
                  '&body=' + encodeURIComponent(buildMessage());
        window.location.href = url;
      });
    }
  }
})();
