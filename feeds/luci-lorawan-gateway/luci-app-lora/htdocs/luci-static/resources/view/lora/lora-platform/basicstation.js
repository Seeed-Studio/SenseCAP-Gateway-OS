'use strict';
'require form';
'require view';
'require uci';
'require ui';
'require request';
'require tools.widgets as widgets';

var CBISimpleFileUpload = form.Value.extend({
    __name__: 'CBI.SimpleFileUpload',

    __init__: function() {
        this.super('__init__', arguments);
        this.root_directory = '/etc/luci-uploads';
    },

    renderWidget: function(section_id, option_index, cfgvalue) {
        var id = this.cbid(section_id);
        var rootDir = this.root_directory;
        var currentValue = cfgvalue || '';
        var selectedFile = null;
        var hiddenInput, fileInput, textSpan, dropzone, container;

        hiddenInput = E('input', {
            'type': 'hidden',
            'name': id,
            'value': currentValue
        });

        fileInput = E('input', {
            'type': 'file',
            'style': 'display:none',
            'change': function(ev) {
                if (ev.target.files.length > 0) {
                    selectedFile = ev.target.files[0];
                    textSpan.textContent = selectedFile.name;
                    dropzone.classList.add('has-file');
                }
            }
        });

        textSpan = E('span', { 'class': 'sfu-text' },
            currentValue ? currentValue.replace(/^.*\//, '') : _('Select file.'));

        dropzone = E('div', {
            'class': 'sfu-dropzone' + (currentValue ? ' has-file' : ''),
            'click': function() { fileInput.click(); }
        }, [textSpan]);

        container = E('div', { 'class': 'sfu-container', 'id': id }, [
            dropzone,
            fileInput,
            E('div', { 'class': 'sfu-buttons' }, [
                E('button', {
                    'class': 'btn sfu-btn-remove',
                    'type': 'button',
                    'click': function(ev) {
                        ev.preventDefault();
                        selectedFile = null;
                        fileInput.value = '';
                        textSpan.textContent = _('Select file.');
                        dropzone.classList.remove('has-file');
                        hiddenInput.value = '';
                    }
                }, _('Remove')),
                E('button', {
                    'class': 'btn sfu-btn-upload',
                    'type': 'button',
                    'click': function(ev) {
                        ev.preventDefault();
                        if (!selectedFile) return;

                        var data = new FormData();
                        data.append('sessionid', L.env.sessionid);
                        data.append('filename', rootDir + '/' + selectedFile.name);
                        data.append('filedata', selectedFile);

                        var btn = ev.target;
                        btn.disabled = true;
                        var origText = btn.textContent;
                        btn.textContent = _('Uploading...');

                        request.post(L.env.cgi_base + '/cgi-upload', data).then(function(res) {
                            var reply = res.json();
                            btn.disabled = false;
                            btn.textContent = origText;
                            if (L.isObject(reply) && reply.failure) {
                                ui.addNotification(null, E('p', _('Upload request failed: %s').format(reply.message)));
                            } else {
                                hiddenInput.value = rootDir + '/' + selectedFile.name;
                                textSpan.textContent = selectedFile.name;
                                dropzone.classList.add('has-file');
                            }
                        }).catch(function(err) {
                            btn.disabled = false;
                            btn.textContent = origText;
                            ui.addNotification(null, E('p', _('Upload failed: %s').format(err.message)));
                        });
                    }
                }, _('Upload'))
            ]),
            hiddenInput
        ]);

        return container;
    },

    formvalue: function(section_id) {
        var node = this.map.findElement('id', this.cbid(section_id));
        if (node) {
            var hidden = node.querySelector('input[type="hidden"]');
            return hidden ? hidden.value : null;
        }
        return null;
    },

    checkValid: function() {
        return true;
    }
});

return view.extend({
    view: function () {
        let s, o;

        var mMap = new form.Map('basicstation');

        s = mMap.section(form.TypedSection, 'auth', 'auth',
            _('General Settings'));
        s.anonymous = true;

        s.tab('general', _('General Settings'));
        s.tab('packet_filter', _('Packet Filter'));
        
        // general tab
        o = s.taboption('general', form.ListValue, 'cred', _('Credentials'),
            _('Credentials for LNS (TC) or CUPS (CUPS)'));
        o.value('tc', _('TC'));
        o.value('cups', _('CUPS'));
        o.default = 'cups';

        o = s.taboption('general', form.ListValue, 'mode', _('Authentication mode'),
            _('Authentication mode for server connection'));
        o.value('no', _('No Authentication'));
        o.value('server', _('TLS Server Authentication'));
        o.value('serverAndClient', _('TLS Server and Client Authentication'));
        o.value('serverAndClientToken', _('TLS Server Authentication and Client Token'));
        o.default = 'serverAndClientToken';

        o = s.taboption('general', form.Value, 'addr', _('Server address'));
        o.optional = false;
        o.rmempty = false;
        o.placeholder = 'eu1.cloud.thethings.network';

        o = s.taboption('general', form.Value, 'port', _('Port'));
        o.optional = false;
        o.rmempty = false;
        o.datatype = 'uinteger';
        o.placeholder = '8887';

        o = s.taboption('general', CBISimpleFileUpload, 'token', _('Authorization token'));
        o.optional = false;
        o.rmempty = false;
        o.root_directory = '/etc/basicstation/token';
        o.depends({ mode: 'serverAndClientToken' });

        o = s.taboption('general', CBISimpleFileUpload, 'key', _('Private station key'),
            _('Please upload a file in .private.key format'));
        o.optional = false;
        o.rmempty = false;
        o.root_directory = '/etc/basicstation/certs/key';
        o.depends({ mode: 'serverAndClient' });

        o = s.taboption('general', CBISimpleFileUpload, 'crt', _('Private station certificate'),
            _('Please upload a file in .cert.pem format'));
        o.optional = false;
        o.rmempty = false;
        o.root_directory = '/etc/basicstation/certs/client';
        o.depends({ mode: "serverAndClient" });

        o = s.taboption('general', CBISimpleFileUpload, 'trust', _('CA certificate'),
            _('Please upload a file in cups.trust format'));
        o.optional = false;
        o.rmempty = false;
        o.root_directory = '/etc/basicstation/certs/ca';
        o.depends({ mode: "no", "!reverse": true });

        // packet_filter tab
        var whitelist_enable = s.taboption('packet_filter', form.Flag, "whitelist_enable", _("Enable White List Mode"),_("OUI filters Join packets; NetID and DevAddr filter uplink packets, they are \"OR\" filters"))
        whitelist_enable.default = 0

        var whitelist_ouis = s.taboption('packet_filter', form.DynamicList, "whitelist_ouis", _("OUI List"), _("Please enter three-byte hexadecimal, eg: SenseCAP Node OUI is '2CF7F1'.Note: Maximum 16 items"))
        whitelist_ouis.datatype = "hexstring"
    
        var whitelist_netids = s.taboption('packet_filter', form.DynamicList, "whitelist_netids", _("Network ID List"), _("Please enter three-byte hexadecimal, eg: SenseCAP TTN NetID is '000013'. Note: Maximum 16 items"))
        whitelist_netids.datatype = "hexstring"

        var whitelist_devaddr_min = s.taboption('packet_filter', form.Value, "whitelist_devaddr_min", _("Devaddr Min"), _("Please enter four-byte hexadecimal, eg: SenseCAP TTN Devaddr min is '27000000'"))
        whitelist_devaddr_min.default = "00000000"
        whitelist_devaddr_min.datatype = "hexstring";

        var whitelist_devaddr_max = s.taboption('packet_filter', form.Value, "whitelist_devaddr_max", _("Devaddr Max"),_("Please enter four-byte hexadecimal, eg: SenseCAP TTN Devaddr min is '2701FFFF'"))
        whitelist_devaddr_max.default = "00000000"
        whitelist_devaddr_max.datatype = "hexstring"

        if (!document.getElementById('basicstation-sfu-style')) {
            var style = document.createElement('style');
            style.id = 'basicstation-sfu-style';
            style.textContent = '.sfu-container{border-radius:10px;padding:16px}.sfu-dropzone{border:2px dashed #cbd5e0;border-radius:6px;padding:16px;text-align:center;cursor:pointer;transition:border-color 0.2s,background-color 0.2s}.sfu-dropzone:hover{border-color:#8FC320;background:#f3f8e8}.sfu-dropzone.has-file{border-style:solid;border-color:#8FC320}.sfu-text{color:#6b7280;font-size:15px}.sfu-dropzone.has-file .sfu-text{color:#2d3748;font-weight:500}.sfu-buttons{display:flex;justify-content:flex-end;gap:10px;margin-top:14px;padding-top:14px;border-top:1px solid #e2e8f0}.sfu-btn-remove,.sfu-btn-upload{color:#fff!important;background:#8898aa;border:none!important;padding:6px 20px;border-radius:4px;font-size:14px;cursor:pointer}.sfu-btn-upload:hover,.sfu-btn-remove:hover{background:#6c7a8d}';
            document.head.appendChild(style);
        }

        return mMap;
    }
});