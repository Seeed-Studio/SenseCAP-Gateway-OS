'use strict';
'require form';
'require view';
'require uci';
'require tools.widgets as widgets';

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

        o = s.taboption('general', form.FileUpload, 'token', _('Authorization token'));
        o.optional = false;
        o.rmempty = false;
        o.root_directory = '/etc/basicstation/token';
        o.depends({ mode: 'serverAndClientToken' });

        o = s.taboption('general', form.FileUpload, 'key', _('Private station key'));
        o.optional = false;
        o.rmempty = false;
        o.root_directory = '/etc/basicstation/certs/key';
        o.depends({ mode: 'serverAndClient' });

        o = s.taboption('general', form.FileUpload, 'crt', _('Private station certificate'));
        o.optional = false;
        o.rmempty = false;
        o.root_directory = '/etc/basicstation/certs/client';
        o.depends({ mode: "serverAndClient" });

        o = s.taboption('general', form.FileUpload, 'trust', _('CA certificate'));
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

        return mMap;
    }
});