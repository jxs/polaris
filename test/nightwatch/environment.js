var assert = require('assert');

module.exports = {
  'Fetch Web API' : function(browser) {
    assert(typeof fetch == "function");
  }
};
