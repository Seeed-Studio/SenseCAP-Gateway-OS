'use strict';
'require view';
'require form';
'require uci';
'require ui';
'require fs';
'require rpc';

/* ------------------------------------------------------------------
 * Build all options for one port Map
 * ------------------------------------------------------------------ */
function buildPortMap(portNum) {
    var portLabel = 'CH' + portNum;
    var sid = 'port' + portNum;

    var m = new form.Map('rs485-module', '');

    /* ---- Section 1: Serial Port Configuration ---- */
    var s1 = m.section(form.NamedSection, sid, 'port', _('Serial Port Configuration'));
    s1.addremove = false;
    var o;

    o = s1.option(form.ListValue, 'baudrate', _('Baud Rate'));
    ['1200','2400','4800','9600','19200','38400','57600','115200'].forEach(function(v) {
        o.value(v, v);
    });
    o.default = '9600';

    o = s1.option(form.ListValue, 'databit', _('Data Bits'));
    ['5','6','7','8'].forEach(function(v) { o.value(v, v); });
    o.default = '8';

    o = s1.option(form.ListValue, 'stopbit', _('Stop Bits'));
    o.value('1', '1'); o.value('2', '2');
    o.default = '1';

    o = s1.option(form.ListValue, 'checkbit', _('Parity'));
    o.value('none', _('None')); o.value('odd', _('Odd')); o.value('even', _('Even'));
    o.default = 'none';

    o = s1.option(form.ListValue, 'flowcontrol', _('Flow Control'));
    o.value('none', _('None')); o.value('rtscts', _('RTS/CTS')); o.value('xonxoff', _('XON/XOFF'));
    o.default = 'none';

    o = s1.option(form.Value, 'timeout', _('Read Timeout (ms)'));
    o.datatype = 'range(100,10000)';
    o.placeholder = '1000'; o.default = '1000';

    /* ---- Section 2: Protocol Configuration ---- */
    var s2 = m.section(form.NamedSection, sid, 'port', _('Protocol Configuration'));
    s2.addremove = false;

    o = s2.option(form.ListValue, 'protocol', _('Protocol'));
    o.value('none',        _('None (Raw Bridge)'));
    o.value('mqtt',        _('MQTT Bridge'));
    o.value('modbus-rtu',  _('Modbus RTU'));
    o.value('bacnet-mstp', _('BACnet MS/TP'));
    o.default = 'none';

    /* ---- MQTT ---- */
    o = s2.option(form.ListValue, 'mqtt_transport', _('Transport'));
    o.depends('protocol', 'mqtt');
    o.value('tcp','TCP'); o.value('ssl','SSL/TLS'); o.value('ws','WebSocket'); o.value('wss','WebSocket Secure');
    o.default = 'tcp';

    o = s2.option(form.Value, 'mqtt_host', _('Server Address'));
    o.depends('protocol', 'mqtt');
    o.datatype = 'or(hostname,ipaddr)'; o.placeholder = 'mqtt.example.com'; o.rmempty = false;

    o = s2.option(form.Value, 'mqtt_port', _('Server Port'));
    o.depends('protocol', 'mqtt');
    o.datatype = 'port'; o.placeholder = '1883'; o.default = '1883';

    o = s2.option(form.Value, 'mqtt_client_id', _('Client ID'));
    o.depends('protocol', 'mqtt');
    o.datatype = 'maxlength(32)'; o.placeholder = 'gateway-rs485-' + portNum;

    o = s2.option(form.Value, 'mqtt_keepalive', _('Keep Alive (s)'));
    o.depends('protocol', 'mqtt');
    o.datatype = 'range(5,120)'; o.placeholder = '30'; o.default = '30';

    o = s2.option(form.Value, 'mqtt_username', _('Username'));
    o.depends('protocol', 'mqtt'); o.optional = true;

    o = s2.option(form.Value, 'mqtt_password', _('Password'));
    o.depends('protocol', 'mqtt'); o.password = true; o.optional = true;

    o = s2.option(form.ListValue, 'mqtt_auth_mode', _('Authentication Mode'));
    o.depends('protocol', 'mqtt');
    o.value('none', _('Username/Password Only')); o.value('tls-server', _('TLS Server Verification'));
    o.value('mutual-tls', _('Mutual TLS')); o.value('token', _('Token Authentication'));
    o.default = 'none';

    o = s2.option(form.FileUpload, 'mqtt_ca_cert', _('CA Certificate'));
    o.depends({ 'protocol': 'mqtt', 'mqtt_auth_mode': 'tls-server' });
    o.depends({ 'protocol': 'mqtt', 'mqtt_auth_mode': 'mutual-tls' }); o.optional = true;

    o = s2.option(form.FileUpload, 'mqtt_client_cert', _('Client Certificate'));
    o.depends({ 'protocol': 'mqtt', 'mqtt_auth_mode': 'mutual-tls' }); o.optional = true;

    o = s2.option(form.FileUpload, 'mqtt_client_key', _('Client Private Key'));
    o.depends({ 'protocol': 'mqtt', 'mqtt_auth_mode': 'mutual-tls' }); o.optional = true;

    o = s2.option(form.Value, 'mqtt_token', _('Access Token'));
    o.depends({ 'protocol': 'mqtt', 'mqtt_auth_mode': 'token' }); o.optional = true;

    o = s2.option(form.Value, 'mqtt_uplink_topic', _('Uplink Topic'));
    o.depends('protocol', 'mqtt');
    o.placeholder = 'rs485/' + portLabel + '/uplink';
    o.default    = 'rs485/' + portLabel + '/uplink';

    o = s2.option(form.Value, 'mqtt_downlink_topic', _('Downlink Topic'));
    o.depends('protocol', 'mqtt');
    o.placeholder = 'rs485/' + portLabel + '/downlink';
    o.default    = 'rs485/' + portLabel + '/downlink';

    o = s2.option(form.ListValue, 'mqtt_qos', _('QoS Level'));
    o.depends('protocol', 'mqtt');
    o.value('0','0 - At most once'); o.value('1','1 - At least once'); o.value('2','2 - Exactly once');
    o.default = '0';

    o = s2.option(form.Flag, 'mqtt_clean_session', _('Clean Session'));
    o.depends('protocol', 'mqtt'); o.default = '1';

    o = s2.option(form.Value, 'mqtt_reconnect_delay', _('Reconnect Delay (s)'));
    o.depends('protocol', 'mqtt');
    o.datatype = 'range(1,120)'; o.placeholder = '5'; o.default = '5';

    /* ---- Modbus RTU ---- */
    o = s2.option(form.Value, 'modbus_device_address', _('Device Address (Slave ID)'),
        _('Value can be entered in hexadecimal (0x) or decimal format.'));
    o.depends('protocol', 'modbus-rtu');
    o.placeholder = '1'; o.default = '1'; o.rmempty = false;

    o = s2.option(form.ListValue, 'modbus_function_code', _('Function Code'));
    o.depends('protocol', 'modbus-rtu');
    o.value('01','01 - Read Coils'); o.value('02','02 - Read Discrete Inputs');
    o.value('03','03 - Read Holding Registers'); o.value('04','04 - Read Input Registers');
    o.value('05','05 - Write Single Coil'); o.value('06','06 - Write Single Register');
    o.value('15','15 - Write Multiple Coils'); o.value('16','16 - Write Multiple Registers');
    o.default = '03';

    o = s2.option(form.Value, 'modbus_register_address', _('Start Register Address'),
        _('Multiple addresses can be separated by commas, e.g. 40001,40010,40020'));
    o.depends('protocol', 'modbus-rtu');
    o.placeholder = '40001'; o.default = '40001'; o.rmempty = false;
    o.validate = function(section_id, value) {
        if (!value || value === '') return _('This field is required.');
        var parts = value.split(',').filter(function(p) { return p.trim() !== ''; });
        if (parts.length === 0) return _('This field is required.');
        for (var i = 0; i < parts.length; i++) {
            var addr = parts[i].trim();
            if (!/^\d+$/.test(addr)) return _('Each address must be a non-negative integer.');
            if (parseInt(addr,10) > 65535) return _('Each address must be between 0 and 65535.');
        }
        return true;
    };

    o = s2.option(form.Value, 'modbus_data_length', _('Register Count'),
        _('Number of registers to read/write. 1 register = 16 bits.'));
    o.depends('protocol', 'modbus-rtu');
    o.datatype = 'range(1,125)'; o.placeholder = '10'; o.default = '10'; o.rmempty = false;

    o = s2.option(form.Flag, 'modbus_enable_crc', _('Enable CRC Check'));
    o.depends('protocol', 'modbus-rtu'); o.default = '1';

    o = s2.option(form.ListValue, 'modbus_work_mode', _('Work Mode'));
    o.depends('protocol', 'modbus-rtu');
    o.value('once', _('Read Once')); o.value('periodic', _('Read Periodic'));
    o.default = 'once';

    o = s2.option(form.Value, 'modbus_poll_interval', _('Measurement Interval (s)'),
        _('Interval between periodic reads. Must be an integer between 1 and 3600.'));
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_work_mode': 'periodic' });
    o.datatype = 'range(1,3600)'; o.default = '3'; o.rmempty = false;

    o = s2.option(form.Value, 'modbus_timeout', _('Timeout (x100ms)'),
        _('Timeout value in units of 100ms. Must be an integer between 1 and 1800.'));
    o.depends('protocol', 'modbus-rtu');
    o.datatype = 'range(1,1800)'; o.default = '10';

    o = s2.option(form.Value, 'modbus_write_value', _('Write Value'));
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '05' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '06' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '15' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '16' });

    o = s2.option(form.Flag, 'modbus_standard_mode', _('Standard Mode'),
        _('Use standard Modbus protocol. Uncheck to use custom hex data mode.'));
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '05' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '06' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '15' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '16' });
    o.default = '1';

    o = s2.option(form.Button, '_modbus_read_btn', _('Read Data'));
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_work_mode': 'once' });
    o.inputtitle = _('Read Data'); o.inputstyle = 'apply';
    o.onclick = L.bind(function(ev) {
        var btn = ev.target;
        var resultArea = document.getElementById('modbus_result_' + portNum);
        btn.disabled = true; btn.innerText = _('Reading...');
        fs.exec('/bin/sh',['-c','rm -f /tmp/rs485/modbus_read_'+portNum+' /tmp/rs485/modbus_result_'+portNum])
        .then(function(){return fs.exec('/bin/sh',['-c','mkdir -p /tmp/rs485 && touch /tmp/rs485/modbus_read_'+portNum]);})
        .then(function(){
            var n=0, timer=setInterval(function(){
                n++;
                L.resolveDefault(fs.read('/tmp/rs485/modbus_result_'+portNum)).then(function(c){
                    if(c){clearInterval(timer);if(resultArea){if(c.startsWith('Error:')){resultArea.value=c;resultArea.style.color='#d00';}else{try{resultArea.value=JSON.stringify(JSON.parse(c),null,4);}catch(e){resultArea.value=c;}resultArea.style.color='';}}btn.disabled=false;btn.innerText=_('Read Data');fs.exec('/bin/sh',['-c','rm -f /tmp/rs485/modbus_read_'+portNum+' /tmp/rs485/modbus_result_'+portNum]);}
                }).catch(function(){if(n>=50){clearInterval(timer);if(resultArea){resultArea.value='Timeout: No response from Modbus device';resultArea.style.color='#d00';}btn.disabled=false;btn.innerText=_('Read Data');}});
            },100);
        });
    }, this);

    o = s2.option(form.Button, '_modbus_write_btn', _('Write Data'));
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '05' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '06' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '15' });
    o.depends({ 'protocol': 'modbus-rtu', 'modbus_function_code': '16' });
    o.inputtitle = _('Write Data'); o.inputstyle = 'apply';
    o.onclick = L.bind(function(ev) {
        var btn = ev.target;
        var resultArea = document.getElementById('modbus_result_' + portNum);
        btn.disabled = true; btn.innerText = _('Writing...');
        fs.exec('/bin/sh',['-c','rm -f /tmp/rs485/modbus_write_'+portNum+' /tmp/rs485/modbus_result_'+portNum])
        .then(function(){return fs.exec('/bin/sh',['-c','mkdir -p /tmp/rs485 && touch /tmp/rs485/modbus_write_'+portNum]);})
        .then(function(){
            var n=0, timer=setInterval(function(){
                n++;
                L.resolveDefault(fs.read('/tmp/rs485/modbus_result_'+portNum)).then(function(c){
                    if(c){clearInterval(timer);if(resultArea){if(c.startsWith('Error:')){resultArea.value=c;resultArea.style.color='#d00';}else{try{resultArea.value=JSON.stringify(JSON.parse(c),null,4);}catch(e){resultArea.value=c;}resultArea.style.color='';}}btn.disabled=false;btn.innerText=_('Write Data');fs.exec('/bin/sh',['-c','rm -f /tmp/rs485/modbus_write_'+portNum+' /tmp/rs485/modbus_result_'+portNum]);}
                }).catch(function(){if(n>=50){clearInterval(timer);if(resultArea){resultArea.value='Timeout: No response from Modbus device';resultArea.style.color='#d00';}btn.disabled=false;btn.innerText=_('Write Data');}});
            },100);
        });
    }, this);

    o = s2.option(form.DummyValue, '_modbus_result', _('Frame Data'));
    o.depends('protocol', 'modbus-rtu'); o.rawhtml = true;
    o.cfgvalue = function() {
        return '<textarea id="modbus_result_' + portNum + '" readonly ' +
            'style="width:100%;min-height:120px;font-family:monospace;font-size:13px;' +
            'padding:8px;border:1px solid #ccc;border-radius:4px;white-space:pre;" ' +
            'placeholder="Frame data..."></textarea>';
    };

    /* ---- BACnet MS/TP ---- */
    o = s2.option(form.Value, 'bacnet_mac_address', _('MAC Address'),
        _('BACnet MS/TP MAC address (0-127).'));
    o.depends('protocol', 'bacnet-mstp');
    o.datatype = 'range(0,127)'; o.placeholder = '1'; o.default = '1';

    o = s2.option(form.Value, 'bacnet_max_master', _('Max Master'),
        _('Maximum MS/TP master address on the network (1-127).'));
    o.depends('protocol', 'bacnet-mstp');
    o.datatype = 'range(1,127)'; o.placeholder = '127'; o.default = '127';

    o = s2.option(form.Value, 'bacnet_max_info_frames', _('Max Info Frames'),
        _('Maximum number of info frames before passing the token.'));
    o.depends('protocol', 'bacnet-mstp');
    o.datatype = 'range(1,100)'; o.placeholder = '1'; o.default = '1';

    o = s2.option(form.Value, 'bacnet_device_instance', _('Device Instance ID'),
        _('Unique BACnet device instance number (0-4194302).'));
    o.depends('protocol', 'bacnet-mstp');
    o.datatype = 'range(0,4194302)'; o.placeholder = '1234'; o.default = '1234';

    o = s2.option(form.Value, 'bacnet_device_name', _('Device Name'),
        _('Human-readable name for this BACnet device.'));
    o.depends('protocol', 'bacnet-mstp');
    o.placeholder = 'SenseCAP Gateway'; o.default = 'SenseCAP Gateway';

    return m;
}

/* ------------------------------------------------------------------
 * Main view  —  3 independent Maps + custom tab UI
 * ------------------------------------------------------------------ */
return view.extend({
    _maps: null,

    load: function() {
        return uci.load('rs485-module').then(function() {
            var needSave = false;
            for (var i = 1; i <= 3; i++) {
                var sid = 'port' + i;
                if (!uci.get('rs485-module', sid)) {
                    uci.add('rs485-module', 'port', sid);
                    needSave = true;
                }
            }
            if (needSave)
                return uci.save().then(function() { return uci.load('rs485-module'); });
        });
    },

    render: function() {
        var self = this;
        self._maps = [buildPortMap(1), buildPortMap(2), buildPortMap(3)];

        return Promise.all(self._maps.map(function(m) { return m.render(); }))
        .then(function(mapEls) {
            /* Hide the empty title h2 from each sub-map */
            mapEls.forEach(function(el, idx) {
                var h2 = el.querySelector('h2');
                if (h2) h2.style.display = 'none';
                /* Only show the first panel initially */
                el.style.display = idx === 0 ? '' : 'none';
            });

            /* Tab bar */
            var tabBar = E('ul', { 'class': 'cbi-tabmenu' });
            mapEls.forEach(function(_, idx) {
                var li = E('li', { 'class': idx === 0 ? 'cbi-tab' : 'cbi-tab-disabled' });
                li.appendChild(E('a', { 'href': 'javascript:void(0)' }, 'CH' + (idx + 1)));
                tabBar.appendChild(li);
            });

            /* Tab switching */
            var tabItems = tabBar.querySelectorAll('li');
            function switchTab(activeIdx) {
                tabItems.forEach(function(t) { t.className = 'cbi-tab-disabled'; });
                tabItems[activeIdx].className = 'cbi-tab';
                mapEls.forEach(function(p) { p.style.display = 'none'; });
                mapEls[activeIdx].style.display = '';
            }
            tabItems.forEach(function(item, idx) {
                item.addEventListener('click', function(e) {
                    e.preventDefault();
                    switchTab(idx);
                });
            });

            /* Wrapper */
            var wrapper = E('div', { 'class': 'cbi-map', 'id': 'rs485-tabs-wrapper' }, [
                E('h2', {}, _('Serial Settings')),
                E('div', { 'class': 'cbi-map-descr' },
                    _('Configure each RS485 port independently.')),
                tabBar
            ]);
            mapEls.forEach(function(el) { wrapper.appendChild(el); });
            return wrapper;
        });
    },

    handleSave: function(ev) {
        /* Parse all maps first (validate + collect UCI changes in memory),
           then save once to avoid duplicate delete RPCs. */
        return Promise.all(this._maps.map(function(m) { return m.parse(); }))
            .then(function() { return uci.save(); });
    },

    handleSaveApply: function(ev, mode) {
        return this.handleSave(ev).then(function() {
            return ui.changes.apply(mode === '0');
        });
    },

    handleReset: null
});
