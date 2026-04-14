'use strict';
'require view';
'require form';
'require uci';

var freq_plan_table = {
    "US915": {
        "plans": [
            {
                "file": "US_902_928_FSB_1",
                "title": "FSB1, channel 0 ~ channel 7, channel 64",
                "mode": ["packet_forwarder"],
                "freqname": "us915_0"
            },
            {
                "file": "US_902_928_FSB_2",
                "title": "FSB2, channel 8 ~ channel 15, channel 65",
                "mode": ["packet_forwarder"],
                "freqname": "us915_1"
            },
            {
                "file": "US_902_928_FSB_3",
                "title": "FSB3, channel 16 ~ channel 23, channel 66",
                "mode": ["packet_forwarder"],
                "freqname": "us915_2"
            },
            {
                "file": "US_902_928_FSB_4",
                "title": "FSB4, channel 24 ~ channel 31, channel 67",
                "mode": ["packet_forwarder"],
                "freqname": "us915_3"
            },
            {
                "file": "US_902_928_FSB_5",
                "title": "FSB5, channel 32 ~ channel 39, channel 68",
                "mode": ["packet_forwarder"],
                "freqname": "us915_4"
            },
            {
                "file": "US_902_928_FSB_6",
                "title": "FSB6, channel 40 ~ channel 47, channel 69",
                "mode": ["packet_forwarder"],
                "freqname": "us915_5"
            },
            {
                "file": "US_902_928_FSB_7",
                "title": "FSB7, channel 48 ~ channel 55, channel 70",
                "mode": ["packet_forwarder"],
                "freqname": "us915_6"
            },
            {
                "file": "US_902_928_FSB_8",
                "title": "FSB8, channel 56 ~ channel 63, channel 71",
                "mode": ["packet_forwarder"],
                "freqname": "us915_7"
            }
        ],
        "device": "US915"
    },
    "AU915": {
        "plans": [
            {
                "file": "AU_915_928_FSB_1",
                "title": "FSB1, channel 0 ~ channel 7, channel 64",
                "mode": ["packet_forwarder"],
                "freqname": "au915_0"
            },
            {
                "file": "AU_915_928_FSB_2",
                "title": "FSB2, channel 8 ~ channel 15, channel 65",
                "mode": ["packet_forwarder"],
                "freqname": "au915_1"
            },
            {
                "file": "AU_915_928_FSB_3",
                "title": "FSB3, channel 16 ~ channel 23, channel 66",
                "mode": ["packet_forwarder"],
                "freqname": "au915_2"
            },
            {
                "file": "AU_915_928_FSB_4",
                "title": "FSB4, channel 24 ~ channel 31, channel 67",
                "mode": ["packet_forwarder"],
                "freqname": "au915_3"
            },
            {
                "file": "AU_915_928_FSB_5",
                "title": "FSB5, channel 32 ~ channel 39, channel 68",
                "mode": ["packet_forwarder"],
                "freqname": "au915_4"
            },
            {
                "file": "AU_915_928_FSB_6",
                "title": "FSB6, channel 40 ~ channel 47, channel 69",
                "mode": ["packet_forwarder"],
                "freqname": "au915_5"
            },
            {
                "file": "AU_915_928_FSB_7",
                "title": "FSB7, channel 48 ~ channel 55, channel 70",
                "mode": ["packet_forwarder"],
                "freqname": "au915_6"
            },
            {
                "file": "AU_915_928_FSB_8",
                "title": "FSB8, channel 56 ~ channel 63, channel 71",
                "mode": ["packet_forwarder"],
                "freqname": "au915_7"
            }
        ],
        "device": "US915"
    },
    "AS923": {
        "plans": [
            {
                "file": "AS_920_923",
                "title": "Asia 920-923 MHz",
                "mode": ["packet_forwarder"],
                "freqname": "as923_2"
            },
            {
                "file": "AS_923_925_TTN",
                "title": "Asia 923-925 MHz",
                "mode": ["packet_forwarder"],
                "freqname": "as923"
            }
            // {
            //     "file": "AS_920_923_LBT",
            //     "title": "Asia 920-923 MHz with LBT",
            //     "mode": ["packet_forwarder"],
            //     "freqname": "as923"
            // },
            // {
            //     "file": "AS_920_923_TTN_JP_1",
            //     "title": "Japan 920-923 MHz with LBT (channels 31-38)",
            //     "mode": ["packet_forwarder"],
            //     "freqname": "as923"
            // },
            // {
            //     "file": "AS_920_923_TTN_JP_2",
            //     "title": "Japan 920-923 MHz with LBT (channels 24-27 and 35-38)",
            //     "mode": ["packet_forwarder"],
            //     "freqname": "as923"
            // },
            // {
            //     "file": "AS_920_923_TTN_JP_3",
            //     "title": "Japan 920-923 MHz with LBT (channels 24-31)",
            //     "mode": ["packet_forwarder"],
            //     "freqname": "as923"
            // }
        ],
        "device": "US915"
    },
    "KR920": {
        "plans": [
            {
                "file": "KR_920_923_TTN",
                "title": "South Korea 920-923 MHz",
                "mode": ["packet_forwarder"],
                "freqname": "kr920"
            }
        ],
        "device": "US915"
    },
    "EU868": {
        "plans": [
            {
                "file": "EU_863_870_TTN",
                "title": "Europe 863-870 MHz",
                "mode": ["packet_forwarder"],
                "freqname": "eu868"
            }
        ],
        "device": "EU868"
    },
    "IN865": {
        "plans": [
            {
                "file": "IN_865_867",
                "title": "India 865-867 MHz",
                "mode": ["packet_forwarder"],
                "freqname": "in865"
            }
        ],
        "device": "EU868"
    },
    "RU864": {
        "plans": [
            {
                "file": "RU_864_870_TTN",
                "title": "Russia 864-870 MHz",
                "mode": ["packet_forwarder"],
                "freqname": "ru864"
            }
        ],
        "device": "EU868"
    },
    "CN470": {
        "plans": [
            {
                "file": "CN_470_510_FSB_1",
                "title": "FSB1, channel 0 ~ channel 7",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_0"
            },
            {
                "file": "CN_470_510_FSB_2",
                "title": "FSB2, channel 8 ~ channel 15",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_1"
            },
            {
                "file": "CN_470_510_FSB_3",
                "title": "FSB3, channel 16 ~ channel 23",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_2"
            },
            {
                "file": "CN_470_510_FSB_4",
                "title": "FSB4, channel 24 ~ channel 31",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_3"
            },
            {
                "file": "CN_470_510_FSB_5",
                "title": "FSB5, channel 32 ~ channel 39",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_4"
            },
            {
                "file": "CN_470_510_FSB_6",
                "title": "FSB6, channel 40 ~ channel 47",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_5"
            },
            {
                "file": "CN_470_510_FSB_7",
                "title": "FSB7, channel 48 ~ channel 55",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_6"
            },
            {
                "file": "CN_470_510_FSB_8",
                "title": "FSB8, channel 56 ~ channel 63",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_7"
            },
            {
                "file": "CN_470_510_FSB_9",
                "title": "FSB9, channel 64 ~ channel 71",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_8"
            },
            {
                "file": "CN_470_510_FSB_10",
                "title": "FSB10, channel 71 ~ channel 79",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_9"
            },
            {
                "file": "CN_470_510_FSB_11",
                "title": "FSB11, channel 80 ~ channel 87",
                "mode": ["sensecap_ttn", "packet_forwarder"],
                "freqname": "cn470_10"
            },
            {
                "file": "CN_470_510_FSB_12",
                "title": "FSB12, channel 88 ~ channel 95",
                "mode": ["packet_forwarder"],
                "freqname": "cn470_11"
            }
        ],
        "device": "CN470"
    }
}

return view.extend({
    setLoRaRegion: function (region) {
        uci.set("packetforwarder", "@sx130x[0]", "region", region);
        uci.set("basicstation", "@sx130x[0]", "region", region);
    },

    setLoRaChannelPlan: function (channelPlan) {
        uci.set("packetforwarder", "@sx130x[0]", "channel_plan", channelPlan);
        // uci.set("basicstation", "@sx130x[0]", "channel_plan", channelPlan);
    },

    load: function () {
        uci.load('basicstation');
        uci.load('packetforwarder');
        return uci.load('lora');
    },

    render: function () {
        var view = this;
        var m = new form.Map('lora', _('Channel Plan'), _('Configure LoRaWAN channel plan.'));
        var s = m.section(form.NamedSection, 'radio', 'radio', _('Channel Plan Settings'));
        s.anonymous = true;
        s.addremove = false;

        var device = uci.get('lora', 'radio', 'device') || 'US915';
        var region = s.option(form.ListValue, 'region', _('Region'));
        var valid_regions = [];

        region.write = function (section_id, value) {
            uci.set('lora', section_id, 'region', value);
            view.setLoRaRegion(value);
        };

        for (var r in freq_plan_table) {
            if (freq_plan_table[r].device == device) {
                valid_regions.push(r);
                region.value(r, r);
            }
        }

        valid_regions.forEach(function (r) {
            var plans = freq_plan_table[r].plans;
            var o = s.option(form.ListValue, 'channel_plan_' + r, _('Channel-plan'), _('Select the channel-plan to use. This must be supported by the selected shield.'));
            o.depends('region', r);


            o.cfgvalue = function (section_id) {
                return uci.get('lora', section_id, 'channel_plan');
            };

            o.write = function (section_id, value) {
                uci.set('lora', section_id, 'channel_plan', value);
                uci.set('lora', section_id, 'freq_plan', freq_plan_table[r].plans.find(p => p.file === value).freqname);
                view.setLoRaChannelPlan(value);
            };

            plans.forEach(function (p) {
                o.value(p.file, p.title);
            });
        });
        return m.render();
    }
});
