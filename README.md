# kristforge

kristforge is an OpenCL-accelerated [krist](http://krist.ceriat.net/) miner, capable of very high speeds. Unlike [turbokrist](https://github.com/apemanzilla/turbokrist), kristforge has full support for vector data types, which can improve speeds.

## Building

kristforge can be built with cmake. You'll need to have OpenCL, OpenSSL, curl, [jsoncpp](https://github.com/open-source-parsers/jsoncpp), [tclap](http://tclap.sourceforge.net/) (only for compiling), and [uwebsockets](https://github.com/uNetworking/uWebSockets) installed. 

Note that currently, Windows is not officially supported due to compiler incompatibilities and library issues. You're welcome to try however, and please submit a PR if you do manage to get it working!
