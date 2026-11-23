'use strict';
'require baseclass';
'require fs';
'require rpc';

var callSystemBoard = rpc.declare({
	object: 'system',
	method: 'board'
});

var callSystemInfo = rpc.declare({
	object: 'system',
	method: 'info'
});

var callLuciVersion = rpc.declare({
	object: 'luci',
	method: 'getVersion'
});

return baseclass.extend({
	title: _('SenseCAP'),

	load: function() {
		var readFile = function(path) {
			return fs.exec('/bin/cat', [path]).then(function(res) {
				return (res.stdout || '').trim();
			}).catch(function() {
				return '';
			});
		};

		var getLTEInfo = fs.exec('/bin/cat', ['/var/run/lte-info.json']).then(function(res) {
			if (res.stdout) {
				try {
					return JSON.parse(res.stdout);
				} catch (e) {
					return {};
				}
			}
			return {};
		}).catch(function() {
			return {};
		});

		return Promise.all([
			L.resolveDefault(callSystemBoard(), {}),
			L.resolveDefault(callSystemInfo(), {}),
			L.resolveDefault(callLuciVersion(), { revision: _('unknown version'), branch: 'LuCI' }),
			readFile('/etc/deviceinfo/sn'),
			readFile('/etc/deviceinfo/eui'),
			readFile('/version.txt'),
			readFile('/etc/deviceinfo/freq_plan'),
			readFile('/etc/deviceinfo/hw_name'),
			readFile('/etc/deviceinfo/os_version'),
			getLTEInfo
		]);
	},

	render: function(data) {
		var boardinfo   = data[0],
		    systeminfo  = data[1],
		    luciversion = data[2],
		    sn          = data[3],
		    eui         = data[4],
		    versionText = data[5],
		    freqPlan    = data[6],
		    hwName      = data[7],
		    osVersion   = data[8],
		    lteInfo     = data[9] || {};

		var lteConnected = (lteInfo.connected === 'true' || lteInfo.connected === true);
		var imei = lteInfo.imei || '';
		var iccid = lteInfo.iccid || '';
		var rssi = lteInfo.rssi || '';

		var firmwareVersion = '';
		if (versionText) {
			var lines = versionText.split('\n');
			for (var i = 0; i < lines.length; i++) {
				if (lines[i].indexOf('Version:') === 0) {
					firmwareVersion = lines[i].replace('Version:', '').trim();
					break;
				}
			}
		}

		luciversion = luciversion.branch + ' ' + luciversion.revision;

		var fields = [
			_('Model'),            hwName || boardinfo.model,
			_('SN'),               sn || '-',
			_('EUI'),              eui || '-',
			_('OS Version'),       osVersion || '-',
			_('Firmware Version'), firmwareVersion || '-',
			_('Build Version'),    (L.isObject(boardinfo.release) ? boardinfo.release.description + ' / ' : '') + (luciversion || ''),
			_('Target Platform'),  (L.isObject(boardinfo.release) ? boardinfo.release.target : ''),
			_('Kernel Version'),   boardinfo.kernel,
			_('LoRaWAN Region'),   freqPlan || '-'
		];

		if (lteConnected) {
			fields.push(_('IMEI'), imei || '-');
			if (iccid !== '')
				fields.push(_('ICCID'), iccid);
			fields.push(_('LTE RSSI'), (rssi !== '') ? rssi + 'dBm' : '-');
		}

		var table = E('table', { 'class': 'table', 'id': 'sensecap-status-table' });

		for (var i = 0; i < fields.length; i += 2) {
			table.appendChild(E('tr', { 'class': 'tr' }, [
				E('td', { 'class': 'td left', 'width': '33%' }, [ fields[i] ]),
				E('td', { 'class': 'td left' }, [ (fields[i + 1] != null) ? fields[i + 1] : '?' ])
			]));
		}

		if (!window._sensecapStatusRefreshTimer) {
			var self = this;
			window._sensecapStatusRefreshTimer = window.setInterval(function() {
				self.load().then(function(data) {
					var container = document.getElementById('sensecap-status-table');
					if (container) {
						var newTable = self.render(data);
						container.parentNode.replaceChild(newTable, container);
					}
				}).catch(function(err) {
					console.log('SenseCAP status refresh error:', err);
				});
			}, 5000);
		}

		return table;
	}
});
