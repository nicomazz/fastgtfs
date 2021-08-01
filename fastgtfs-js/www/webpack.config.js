const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
    entry: ["./bootstrap.ts"],
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                use: 'ts-loader',
                exclude: /node_modules/,
            },
        ],
    },
    devtool: 'inline-source-map',

    resolve: {
        extensions: ['.tsx', '.ts', '.js'],
    },
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "bootstrap.js",
    },
    experiments: {asyncWebAssembly: true},
    mode: 'production',
    plugins: [
        new CopyWebpackPlugin({
            patterns: [
                {from: 'index.html', to: 'index.html'},
                {from: 'gtfs_serialized.zip', to: 'gtfs_serialized.zip'},
            ]
        })
    ],
};
