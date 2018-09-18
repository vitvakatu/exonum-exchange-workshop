'use strict';

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.send = send;
exports.sendQueue = sendQueue;

var _axios = require('axios');

var _axios2 = _interopRequireDefault(_axios);

var _helpers = require('../helpers');

var _validate = require('../types/validate');

var validate = _interopRequireWildcard(_validate);

var _message = require('../types/message');

function _interopRequireWildcard(obj) { if (obj && obj.__esModule) { return obj; } else { var newObj = {}; if (obj != null) { for (var key in obj) { if (Object.prototype.hasOwnProperty.call(obj, key)) newObj[key] = obj[key]; } } newObj.default = obj; return newObj; } }

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

var ATTEMPTS = 10;
var ATTEMPT_TIMEOUT = 500;

/**
 * Send transaction to the blockchain
 * @param {string} transactionEndpoint
 * @param {string} explorerBasePath
 * @param {Object} data
 * @param {string} signature
 * @param {NewMessage} type
 * @param {number} attempts
 * @param {number} timeout
 * @return {Promise}
 */
function send(transactionEndpoint, explorerBasePath, data, signature, type, attempts, timeout) {
  if (typeof transactionEndpoint !== 'string') {
    throw new TypeError('Transaction endpoint of wrong data type is passed. String is required.');
  }
  if (typeof explorerBasePath !== 'string') {
    throw new TypeError('Explorer base path endpoint of wrong data type is passed. String is required.');
  }
  if (!(0, _helpers.isObject)(data)) {
    throw new TypeError('Data of wrong data type is passed. Object is required.');
  }
  if (!validate.validateHexadecimal(signature, 64)) {
    throw new TypeError('Signature of wrong type is passed. Hexadecimal expected.');
  }
  if (!(0, _message.isInstanceofOfNewMessage)(type)) {
    throw new TypeError('Transaction of wrong type is passed.');
  }
  if (typeof attempts !== 'undefined') {
    if (isNaN(parseInt(attempts)) || attempts < 0) {
      throw new TypeError('Attempts of wrong type is passed.');
    }
  } else {
    attempts = ATTEMPTS;
  }
  if (typeof timeout !== 'undefined') {
    if (isNaN(parseInt(timeout)) || timeout <= 0) {
      throw new TypeError('Timeout of wrong type is passed.');
    }
  } else {
    timeout = ATTEMPT_TIMEOUT;
  }

  type.signature = signature;
  var hash = type.hash(data);

  return _axios2.default.post(transactionEndpoint, {
    protocol_version: type.protocol_version,
    service_id: type.service_id,
    message_id: type.message_id,
    body: data,
    signature: signature
  }).then(function () {
    if (attempts > 0) {
      var count = attempts;
      return function attempt() {
        if (count-- > 0) {
          return _axios2.default.get('' + explorerBasePath + hash).then(function (response) {
            if (response.data.type === 'committed') {
              return hash;
            }
            return new Promise(function (resolve) {
              setTimeout(resolve, timeout);
            }).then(attempt);
          }).catch(function () {
            if (count > 0) {
              return new Promise(function (resolve) {
                setTimeout(resolve, timeout);
              }).then(attempt);
            }
            throw new Error('The request failed or the blockchain did not respond.');
          });
        } else {
          throw new Error('Transaction is not accepted to the block.');
        }
      }();
    }
  });
}

/**
 * Send transaction to the blockchain
 * @param {string} transactionEndpoint
 * @param {string} explorerBasePath
 * @param {Array} transactions
 * @param {number} attempts
 * @param {number} timeout
 * @return {Promise}
 */
function sendQueue(transactionEndpoint, explorerBasePath, transactions, attempts, timeout) {
  var index = 0;
  var responses = [];

  return function shift() {
    var transaction = transactions[index++];

    return send(transactionEndpoint, explorerBasePath, transaction.data, transaction.signature, transaction.type, attempts, timeout).then(function (response) {
      responses.push(response);
      if (index < transactions.length) {
        return shift();
      } else {
        return responses;
      }
    });
  }();
}
