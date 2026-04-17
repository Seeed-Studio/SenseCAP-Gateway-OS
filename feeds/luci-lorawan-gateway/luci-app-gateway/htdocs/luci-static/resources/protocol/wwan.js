'use strict';
'require form';
'require network';

return network.registerProtocol('wwan', {
	getI18n: function() {
		return _('WWAN');
	},

	renderFormOptions: function(s) {
		var o;

		o = s.taboption('general', form.Value, 'apn', _('APN'));
		o.optional = true;

		o = s.taboption('general', form.Value, 'pincode', _('PIN'));
		o.optional = true;
		o.datatype = 'and(uinteger,minlength(4),maxlength(8))';
	}
});
