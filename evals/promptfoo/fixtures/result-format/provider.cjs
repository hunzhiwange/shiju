module.exports = class FixedProvider {
  id() { return 'local-fixed'; }
  async callApi() {
    return { output: 'fixed', tokenUsage: { total: 3, prompt: 2, completion: 1 } };
  }
};
