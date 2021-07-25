const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
  entry: ["./bootstrap.js"],
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
  },
  mode: 'production',
  plugins: [
    new CopyWebpackPlugin(['index.html']),
    new CopyWebpackPlugin(['gtfs_serialized.zip']),
    new CopyWebpackPlugin(['index.js']),
  ],
};
