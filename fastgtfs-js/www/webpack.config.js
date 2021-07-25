const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
    entry: ["./bootstrap.js"],
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "bootstrap.js",
    },
    experiments: {asyncWebAssembly: true},
    mode: 'production',
    plugins: [
        new CopyWebpackPlugin({
            patterns: [
                {from: 'index.html', to: 'dist/index.html'},
                {from: 'gtfs_serialized.zip', to: 'dist/gtfs_serialized.zip'},
                {from: 'index.js', to: 'dist/index.js'},

            ]
        })
    ],
};
