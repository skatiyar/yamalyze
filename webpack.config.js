const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

module.exports = {
  entry: './index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'index.js',
  },
  plugins: [
    new HtmlWebpackPlugin({
      filename: 'index.html',
      template: './static/index.html',
      chunks: ['main'],
      favicon: './static/favicon.ico',
    }),
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, ".")
    }),
  ],
  mode: process.env.NODE_ENV || 'development',
  experiments: {
    asyncWebAssembly: true
  }
};
