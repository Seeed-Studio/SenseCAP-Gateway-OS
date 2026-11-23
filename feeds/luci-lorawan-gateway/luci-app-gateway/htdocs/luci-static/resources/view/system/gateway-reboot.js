'use strict';
'require view';
'require rpc';
'require ui';
'require uci';

var callReboot = rpc.declare({
	object: 'system',
	method: 'reboot',
	expect: { result: 0 }
});

var callRebootHost = rpc.declare({
	object: 'sensecap',
	method: 'reboot_host',
	expect: { result: 0 }
});

var callShutdownHost = rpc.declare({
	object: 'sensecap',
	method: 'shutdown_host',
	expect: { result: 0 }
});

return view.extend({
	load: function() {
		return uci.changes();
	},

	render: function(changes) {
		var body = E([
			E('h2', _('Reboot')),
			E('p', {}, _('Reboots the operating system of your device'))
		]);

		for (var config in (changes || {})) {
			body.appendChild(E('p', { 'class': 'alert-message warning' },
				_('Warning: There are unsaved changes that will get lost on reboot!')));
			break;
		}

		body.appendChild(E('hr'));
		body.appendChild(E('button', {
			'class': 'cbi-button cbi-button-action important',
			'click': ui.createHandlerFn(this, 'handleReboot')
		}, _('Perform reboot')));

		body.appendChild(E('span', {}, ' '));

		body.appendChild(E('button', {
			'class': 'cbi-button cbi-button-action important',
			'click': ui.createHandlerFn(this, 'handleRebootHost')
		}, _('Perform host reboot')));

		body.appendChild(E('span', {}, ' '));

		body.appendChild(E('button', {
			'class': 'cbi-button cbi-button-action important',
			'click': ui.createHandlerFn(this, 'handleShutdownHost')
		}, _('Perform host shutdown')));

		return body;
	},

	handleReboot: function(ev) {
		return callReboot().then(function(res) {
			if (res != 0) {
				L.ui.addNotification(null, E('p', _('The reboot command failed with code %d').format(res)));
				L.raise('Error', 'Reboot failed');
			}

			L.ui.showModal(_('Rebooting…'), [
				E('p', { 'class': 'spinning' }, _('Waiting for device...'))
			]);

			window.setTimeout(function() {
				L.ui.showModal(_('Rebooting…'), [
					E('p', { 'class': 'spinning alert-message warning' },
						_('Device unreachable! Still waiting for device...'))
				]);
			}, 150000);

			L.ui.awaitReconnect();
		})
		.catch(function(e) { L.ui.addNotification(null, E('p', e.message)) });
	},

	handleRebootHost: function(ev) {
		return callRebootHost().then(function(res) {
			if (res != 0) {
				L.ui.addNotification(null, E('p', _('The host reboot command failed with code %d').format(res)));
				L.raise('Error', 'Host Reboot failed');
			}

			L.ui.showModal(_('Rebooting Host…'), [
				E('p', { 'class': 'spinning' }, _('Waiting for device...'))
			]);

			window.setTimeout(function() {
				L.ui.showModal(_('Rebooting Host…'), [
					E('p', { 'class': 'spinning alert-message warning' },
						_('Device unreachable! Still waiting for device...'))
				]);
			}, 150000);

			L.ui.awaitReconnect();
		})
		.catch(function(e) { L.ui.addNotification(null, E('p', e.message)) });
	},

	handleShutdownHost: function(ev) {
		return callShutdownHost().then(function(res) {
			if (res != 0) {
				L.ui.addNotification(null, E('p', _('The host shutdown command failed with code %d').format(res)));
				L.raise('Error', 'Host Shutdown failed');
			}

			L.ui.showModal(_('Shutting down Host…'), [
				E('p', { 'class': 'spinning' }, _('Device is shutting down...'))
			]);
		})
		.catch(function(e) { L.ui.addNotification(null, E('p', e.message)) });
	},

	handleSaveApply: null,
	handleSave: null,
	handleReset: null
});
