const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');
const webpack = require("webpack");

module.exports = {
    entry: ["./src/index.ts"],
    module: {
        rules: [
            {
                test: /\.(js|ts)x*$/,
                exclude: /node_modules/,
                loader: "babel-loader",
                options: { presets: ["@babel/env"] }
            },
            {
                test: /\.css$/,
                use: ["style-loader", "css-loader"]
            }
        ],
    },
    devtool: 'inline-source-map',
    devServer: {
        static: path.resolve(__dirname, './dist')
    },
    resolve: {
        extensions: ['.tsx', '.ts', '.js'],
    },
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "bundle.js",
    },
    performance: {
        hints: false,
        maxEntrypointSize: 512000,
        maxAssetSize: 512000
    },
    experiments: {asyncWebAssembly: true},
    mode: 'production',
    plugins: [
        new webpack.HotModuleReplacementPlugin(),
        new CopyWebpackPlugin({
            patterns: [
                {from: 'index.html', to: 'index.html'},
                {from: 'gtfs_serialized.zip', to: 'gtfs_serialized.zip'},
            ]
        })
    ],
};
