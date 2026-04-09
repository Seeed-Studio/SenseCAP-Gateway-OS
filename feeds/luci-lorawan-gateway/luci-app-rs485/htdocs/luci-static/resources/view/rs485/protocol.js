'use strict';
'require view';
'require form';
'require fs';
'require ui';
'require uci';
'require rpc';

var callFileRead = rpc.declare({
    object: 'file',
    method: 'read',
    params: ['path'],
    expect: { data: '' }
});

return view.extend({
    load: function() {
        return Promise.all([
            L.resolveDefault(fs.stat('/tmp/rs485'), null)
        ]);
    },

    render: function(data) {
        var m, s, o;

        m = new form.Map('rs485-module', _('Protocol Settings'),
            _('Configure RS485 protocol settings'));

        s = m.section(form.TypedSection, 'protocol', _('Protocol Configuration'));
        s.anonymous = true;
        s.addremove = false;

        o = s.option(form.Button, '_toggle_protocol', _('Protocol Status'));
        o.inputtitle = function() {
            var enabled = uci.get('rs485-module', 'protocol', 'enabled');
            return enabled === '1' ? _('Disable Protocol') : _('Enable Protocol');
        };
        o.inputstyle = function() {
            var enabled = uci.get('rs485-module', 'protocol', 'enabled');
            return enabled === '1' ? 'reset' : 'apply';
        };
        o.onclick = function(ev) {
            var currentEnabled = uci.get('rs485-module', 'protocol', 'enabled');
            
            if (currentEnabled !== '1') {
                var serialEnabled = uci.get('rs485-module', 'serial', 'enabled');
                if (serialEnabled !== '1') {
                    ui.showModal(_('Cannot Enable Protocol'), [
                        E('p', _('Please enable Serial Port first before enabling Protocol.')),
                        E('div', { 'style': 'display: flex; justify-content: space-between; margin-top: 10px;' }, [
                            E('button', {
                                'class': 'cbi-button cbi-button-primary',
                                'click': function() {
                                    ui.hideModal();
                                    window.location.href = '/cgi-bin/luci/admin/rs485/serial';
                                }
                            }, _('Go to Serial Settings')),
                            E('button', {
                                'class': 'cbi-button',
                                'click': ui.hideModal
                            }, _('Cancel'))
                        ])
                    ]);
                    return;
                }
                uci.set('rs485-module', 'protocol', 'enabled', '1');
                ev.target.textContent = _('Disable Protocol');
                ev.target.className = 'cbi-button cbi-button-reset';
            } else {
                uci.set('rs485-module', 'protocol', 'enabled', '0');
                ev.target.textContent = _('Enable Protocol');
                ev.target.className = 'cbi-button cbi-button-apply';
            }
        };
        o.description = _('When enabled, decodes data according to the specified protocol.');

        o = s.option(form.ListValue, 'type', _('Protocol Type'));
        o.value('modbus-rtu', 'Modbus RTU');
        o.value('bacnet-mstp', 'BACnet MS/TP');
        o.default = 'modbus-rtu';
        o.validate = function(section_id, value) {
            if (value === 'bacnet-mstp') {
                return _('BACnet MS/TP has a dedicated configuration page. Please use the BACnet menu.');
            }
            return true;
        };

        // Link to dedicated BACnet configuration page
        o = s.option(form.DummyValue, '_bacnet_notice', _('Notice'));
        o.depends('type', 'bacnet-mstp');
        o.rawhtml = true;
        o.cfgvalue = function() {
            return _('BACnet MS/TP now has a dedicated configuration interface. <br><a href="/cgi-bin/luci/admin/bacnet/general" class="cbi-button cbi-button-apply" style="display:inline-block;margin-top:6px;">Go to BACnet Configuration</a>');
        };

        o = s.option(form.Value, 'device_address', _('Device Address (Slave ID)'), _('Value can be entered in hexadecimal (0x) or decimal format.'));
        o.depends('type', 'modbus-rtu');
        o.placeholder = '1';
        o.default = '1';
        o.rmempty = false;

        o = s.option(form.ListValue, 'function_code', _('Function Code'));
        o.depends('type', 'modbus-rtu');
        o.value('01', '01 - Read Coils');
        o.value('02', '02 - Read Discrete Inputs');
        o.value('03', '03 - Read Holding Registers');
        o.value('04', '04 - Read Input Registers');
        o.value('05', '05 - Write Single Coil');
        o.value('06', '06 - Write Single Register');
        o.value('15', '15 - Write Multiple Coils');
        o.value('16', '16 - Write Multiple Registers');
        o.default = '03';

        o = s.option(form.Value, 'register_address', _('Start Register Address'),
            _('Multiple addresses can be separated by commas, e.g. 40001,40010,40020'));
        o.depends('type', 'modbus-rtu');
        o.placeholder = '40001,40010,40020';
        o.default = '40001';
        o.rmempty = false;
        o.validate = function(section_id, value) {
            if (!value || value === '')
                return _('This field is required.');
            var parts = value.split(',').filter(function(s) { return s.trim() !== ''; });
            if (parts.length === 0)
                return _('This field is required.');
            for (var i = 0; i < parts.length; i++) {
                var addr = parts[i].trim();
                if (!/^\d+$/.test(addr))
                    return _('Each address must be a non-negative integer.');
                var num = parseInt(addr, 10);
                if (num < 0 || num > 65535)
                    return _('Each address must be between 0 and 65535.');
            }
            return true;
        };

        o = s.option(form.Value, 'data_length', _('Register Count'), _('Number of registers to read/write. 1 register = 16 bits.'));
        o.depends('type', 'modbus-rtu');
        o.datatype = 'range(1,125)';
        o.placeholder = '10';
        o.default = '10';
        o.rmempty = false;

        o = s.option(form.Flag, 'enable_crc', _('Enable CRC Check'));
        o.depends('type', 'modbus-rtu');
        o.default = '1';
        o.rmempty = false;

        o = s.option(form.ListValue, 'work_mode', _('Work Mode'));
        o.depends({'type': 'modbus-rtu', 'function_code': '01'});
        o.depends({'type': 'modbus-rtu', 'function_code': '02'});
        o.depends({'type': 'modbus-rtu', 'function_code': '03'});
        o.depends({'type': 'modbus-rtu', 'function_code': '04'});
        o.value('once', _('Read Once'));
        o.value('periodic', _('Read Periodic'));
        o.default = 'once';

        o = s.option(form.Value, 'poll_interval', _('Measurement Interval (x1s)'),
            _('Interval between periodic reads. Must be an integer between 1 and 3600.'));
        o.datatype = 'range(1,3600)';
        o.default = '3';
        o.rmempty = false;
        o.depends({'type': 'modbus-rtu', 'function_code': '01', 'work_mode': 'periodic'});
        o.depends({'type': 'modbus-rtu', 'function_code': '02', 'work_mode': 'periodic'});
        o.depends({'type': 'modbus-rtu', 'function_code': '03', 'work_mode': 'periodic'});
        o.depends({'type': 'modbus-rtu', 'function_code': '04', 'work_mode': 'periodic'});

        o = s.option(form.Button, '_show_frame_btn', _('Read Data'));
        o.inputtitle = _('Read Data');
        o.inputstyle = 'apply';
        o.depends({'type': 'modbus-rtu', 'function_code': '01', 'work_mode': 'once'});
        o.depends({'type': 'modbus-rtu', 'function_code': '02', 'work_mode': 'once'});
        o.depends({'type': 'modbus-rtu', 'function_code': '03', 'work_mode': 'once'});
        o.depends({'type': 'modbus-rtu', 'function_code': '04', 'work_mode': 'once'});
        o.description = _('Click to read data from the Modbus device once.');
        o.onclick = L.bind(function (ev) {
            var btn = ev.target;
            var resultArea = document.getElementById('modbus_result');
            
            // Check if protocol is enabled first
            return uci.load('rs485-module').then(function() {
                var protocolEnabled = uci.get('rs485-module', 'protocol', 'enabled');
                
                if (protocolEnabled !== '1') {
                    if (resultArea) {
                        resultArea.value = 'Error: Protocol processing is not enabled. Please enable it and save first.';
                        resultArea.style.color = '#d00';
                    }
                    return Promise.reject('Protocol not enabled');
                }
                
                btn.disabled = true;
                btn.innerText = _('Reading...');
                
                // Clean up old files first
                return fs.exec('/bin/sh', ['-c', 'rm -f /tmp/rs485/modbus_read /tmp/rs485/modbus_result'])
                    .then(function() {
                        // Create trigger file
                        return fs.exec('/bin/sh', ['-c', 'mkdir -p /tmp/rs485 && touch /tmp/rs485/modbus_read']);
                    })
                    .then(function () {
                        // Poll for result file (max 5 seconds)
                        var pollCount = 0;
                        var pollInterval = setInterval(function () {
                            pollCount++;

                            L.resolveDefault(fs.read('/tmp/rs485/modbus_result'))
                                .then(function (content) {
                                    if (content) {
                                        clearInterval(pollInterval);
                                        if (resultArea) {
                                            if (content.startsWith('Error:')) {
                                                resultArea.value = content;
                                                resultArea.style.color = '#d00';
                                            } else {
                                                try {
                                                    var parsed = JSON.parse(content);
                                                    resultArea.value = JSON.stringify(parsed, null, 4);
                                                } catch(e) {
                                                    resultArea.value = content;
                                                }
                                                resultArea.style.color = '';
                                            }
                                        }
                                        btn.disabled = false;
                                        btn.innerText = _('Read Data');
                                        // Clean up files
                                        fs.exec('/bin/sh', ['-c', 'rm -f /tmp/rs485/modbus_read /tmp/rs485/modbus_result']);
                                    }
                                })
                                .catch(function (err) {
                                    if (pollCount >= 50) {
                                        clearInterval(pollInterval);
                                        if (resultArea) {
                                            resultArea.value = 'Timeout: No response from Modbus device';
                                            resultArea.style.color = '#d00';
                                        }
                                        btn.disabled = false;
                                        btn.innerText = _('Read Data');
                                        // Clean up files
                                        fs.exec('/bin/sh', ['-c', 'rm -f /tmp/rs485/modbus_read /tmp/rs485/modbus_result']);
                                    }
                                });
                        }, 100);
                    });
            }).catch(function(err) {
                if (resultArea) {
                    resultArea.value = 'Error: ' + (err.message || err);
                    resultArea.style.color = '#d00';
                }
                btn.disabled = false;
                btn.innerText = _('Read Data');
            });
        }, this);

        o = s.option(form.Value, 'timeout', _('Timeout (x100ms)'),
            _('Timeout value in units of 100ms. Must be an integer between 1 and 1800.'));
        o.datatype = 'range(1,1800)';
        o.default = '10';
        o.rmempty = false;
        o.depends({'type': 'modbus-rtu', 'function_code': '01'});
        o.depends({'type': 'modbus-rtu', 'function_code': '02'});
        o.depends({'type': 'modbus-rtu', 'function_code': '03'});
        o.depends({'type': 'modbus-rtu', 'function_code': '04'});

        // Write Data button (for function codes 05, 06, 15, 16)
        o = s.option(form.Button, '_write_data_btn', _('Write Data'));
        o.depends({'type': 'modbus-rtu', 'function_code': '05'});
        o.depends({'type': 'modbus-rtu', 'function_code': '06'});
        o.depends({'type': 'modbus-rtu', 'function_code': '15'});
        o.depends({'type': 'modbus-rtu', 'function_code': '16'});
        o.inputtitle = _('Write Data');
        o.inputstyle = 'apply';
        o.onclick = L.bind(function (ev) {
            var btn = ev.target;
            var resultArea = document.getElementById('modbus_result');
            
            // Check if protocol is enabled first
            return uci.load('rs485-module').then(function() {
                var protocolEnabled = uci.get('rs485-module', 'protocol', 'enabled');
                
                if (protocolEnabled !== '1') {
                    if (resultArea) {
                        resultArea.value = 'Error: Protocol processing is not enabled. Please enable it and save first.';
                        resultArea.style.color = '#d00';
                    }
                    return Promise.reject('Protocol not enabled');
                }
                
                btn.disabled = true;
                btn.innerText = _('Writing...');
                
                // Clean up old files first
                return fs.exec('/bin/sh', ['-c', 'rm -f /tmp/rs485/modbus_write /tmp/rs485/modbus_result'])
                    .then(function() {
                        return fs.exec('/bin/sh', ['-c', 'mkdir -p /tmp/rs485 && touch /tmp/rs485/modbus_write']);
                    })
                    .then(function () {
                        var pollCount = 0;
                        // Poll for result file (max 5 seconds)
                        var pollInterval = setInterval(function () {
                            pollCount++;

                            L.resolveDefault(fs.read('/tmp/rs485/modbus_result'))
                                .then(function (content) {
                                    if (content) {
                                        clearInterval(pollInterval);
                                        if (resultArea) {
                                            if (content.startsWith('Error:')) {
                                                resultArea.value = content;
                                                resultArea.style.color = '#d00';
                                            } else {
                                                try {
                                                    var parsed = JSON.parse(content);
                                                    resultArea.value = JSON.stringify(parsed, null, 4);
                                                } catch(e) {
                                                    resultArea.value = content;
                                                }
                                                resultArea.style.color = '';
                                            }
                                        }
                                        btn.disabled = false;
                                        btn.innerText = _('Write Data');
                                        // Clean up files
                                        fs.exec('/bin/sh', ['-c', 'rm -f /tmp/rs485/modbus_write /tmp/rs485/modbus_result']);
                                    }
                                })
                                .catch(function (err) {
                                    if (pollCount >= 50) {
                                        clearInterval(pollInterval);
                                        if (resultArea) {
                                            resultArea.value = 'Timeout: No response from Modbus device';
                                            resultArea.style.color = '#d00';
                                        }
                                        btn.disabled = false;
                                        btn.innerText = _('Write Data');
                                        // Clean up files
                                        fs.exec('/bin/sh', ['-c', 'rm -f /tmp/rs485/modbus_write /tmp/rs485/modbus_result']);
                                    }
                                });
                        }, 100);
                    });
            }).catch(function(err) {
                if (resultArea) {
                    resultArea.value = 'Error: ' + (err.message || err);
                    resultArea.style.color = '#d00';
                }
                btn.disabled = false;
                btn.innerText = _('Write Data');
            });
        }, this);

        // Write data value input
        o = s.option(form.Value, 'write_value', _('Write Value'));
        o.depends({'type': 'modbus-rtu', 'function_code': '05'});
        o.depends({'type': 'modbus-rtu', 'function_code': '06'});
        o.depends({'type': 'modbus-rtu', 'function_code': '15'});
        o.depends({'type': 'modbus-rtu', 'function_code': '16'});

        // Standard mode checkbox
        o = s.option(form.Flag, 'standard_mode', _('Standard Mode'),
            _('Use standard Modbus protocol. Uncheck to use custom hex data mode.'));
        o.default = '1';
        o.depends({'type': 'modbus-rtu', 'function_code': '05'});
        o.depends({'type': 'modbus-rtu', 'function_code': '06'});
        o.depends({'type': 'modbus-rtu', 'function_code': '15'});
        o.depends({'type': 'modbus-rtu', 'function_code': '16'});

        // Result display area
        o = s.option(form.DummyValue, '_result_display', _('Frame Data'));
        o.depends('type', 'modbus-rtu');
        o.rawhtml = true;
        o.cfgvalue = function() {
            return '<div style="margin-top:10px;">' +
                   '<textarea id="modbus_result" readonly style="width:100%;min-height:150px;font-family:monospace;font-size:13px;padding:8px;border:1px solid #ccc;border-radius:4px;white-space:pre;tab-size:4;" placeholder="Frame data..."></textarea>' +
                   '</div>';
        };

        return m.render().then(function(renderedNode) {
            // Start periodic read timer if in periodic mode
            var periodicTimer = null;
            
            function startPeriodicRead() {
                if (periodicTimer) {
                    clearInterval(periodicTimer);
                    periodicTimer = null;
                }
                
                return uci.load('rs485-module').then(function() {
                    var workMode = uci.get('rs485-module', 'protocol', 'work_mode');
                    var functionCode = uci.get('rs485-module', 'protocol', 'function_code');
                    var pollInterval = parseInt(uci.get('rs485-module', 'protocol', 'poll_interval')) || 10;
                    var protocolEnabled = uci.get('rs485-module', 'protocol', 'enabled');
                    
                    // Only start timer for periodic mode with read function codes (01-04)
                    if (protocolEnabled === '1' && workMode === 'periodic' && ['01', '02', '03', '04'].indexOf(functionCode) !== -1) {
                        periodicTimer = setInterval(function() {
                            var resultArea = document.getElementById('modbus_result');
                            if (!resultArea) return;
                            
                            // Create trigger file
                            fs.exec('/bin/sh', ['-c', 'mkdir -p /tmp/rs485 && touch /tmp/rs485/modbus_read'])
                                .then(function() {
                                    // Poll for result file (max 5 seconds)
                                    var pollCount = 0;
                                    var pollInterval = setInterval(function() {
                                        pollCount++;
                                        
                                        L.resolveDefault(fs.read('/tmp/rs485/modbus_result'))
                                            .then(function(content) {
                                                if (content) {
                                                    clearInterval(pollInterval);
                                                    if (content.startsWith('Error:')) {
                                                        resultArea.value = content;
                                                        resultArea.style.color = '#d00';
                                                    } else {
                                                        try {
                                                            var parsed = JSON.parse(content);
                                                            resultArea.value = JSON.stringify(parsed, null, 4);
                                                        } catch(e) {
                                                            resultArea.value = content;
                                                        }
                                                        resultArea.style.color = '';
                                                    }
                                                    fs.exec('/bin/sh', ['-c', 'rm -f /tmp/rs485/modbus_read /tmp/rs485/modbus_result']);
                                                }
                                            })
                                            .catch(function() {
                                                if (pollCount >= 50) {
                                                    clearInterval(pollInterval);
                                                    fs.exec('/bin/sh', ['-c', 'rm -f /tmp/rs485/modbus_read /tmp/rs485/modbus_result']);
                                                }
                                            });
                                    }, 100);
                                });
                        }, pollInterval * 1000);
                    }
                });
            }
            
            // Start periodic read on initial render
            startPeriodicRead();
            
            // Restart periodic read when configuration changes
            var originalHandleSave = m.handleSave;
            m.handleSave = function() {
                return originalHandleSave.apply(this, arguments).then(function(result) {
                    startPeriodicRead();
                    return result;
                });
            };
            
            // Clean up timer when page unloads
            window.addEventListener('beforeunload', function() {
                if (periodicTimer) {
                    clearInterval(periodicTimer);
                }
            });
            
            return renderedNode;
        });
    }
});
