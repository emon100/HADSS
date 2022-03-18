const net = require('net');
const http = require('http');
const fs = require('fs/promises');
const crypto = require('crypto');
//Transport layer
// base64
// GET <object name>\n
// POST <data>\n
net.createServer(socket => {
    const storage = Object.create(null);
    function processLine(i) {
        let res = "";
        try{
            const tokens = i.split(" ");
            if (tokens.length !== 2) {
                throw "Command format not correct.";
            }
            if (tokens[0] === "GET") {
                res=`${tokens[1]}`
            } else if (tokens[0] === "POST") {
                const key = crypto.randomUUID().slice(0,5);
                storage[key] = tokens[1];
                res = key;
            } else {
                throw "Command not found";
            }
        }catch(e){
            res="ERROR"
        }
        return res + "\n";
    }
    // Socket can be wrapped by a stream.
    let buf = "";
    socket.on('data', (data)=>{
        buf+=data;
        const lines = buf.split("\n");
        for (let i = 0; i < lines.length -1;++i){
            socket.write(processLine(lines[i]));
        }
        buf = lines[lines.length-1];
    });
}).listen('12000');

//Index layer
// GET
// POST <hardware_status_in_json>
net.createServer(socket => {
    const status = Object.create(null);
    status[0] = ["localhost:12002"];
    function processLine(i) {
        const tokens = i.split(" ");
        let res = "";
        if (tokens[0] === "GET") {
            res+=`${JSON.stringify(status)}`
        } else if (tokens[0] === "POST") {
            status[0].push(JSON.parse(tokens[1]))
        } else {
            res+="ERROR"
        }
        res+="\n"
        return res;
    }
    let buf = "";
    socket.on('data', (data)=>{
        buf+=data;
        const lines = buf.split("\n");
        for (let i = 0; i < lines.length -1;++i){
            socket.write(processLine(lines[i]));
        }
        buf = lines[lines.length-1];
    });
}).listen('12001');

//Storage layer
//GET <HASH>
//POST <HASH> <DATA>
net.createServer(async socket => {
    const status = Object.create(null);
    status[0] = ["localhost:12002"];
    async function processLine(line) {
        const tokens = line.split(" ");
        let res = "";
        if (tokens[0] === "GET") {
            const filepath = tokens[1];
            await fs.readFile(filepath).then((buf) => {
                res = "OK "+buf;
            }).catch((e)=>{
                res = "FAIL: "+e;
            });
        } else if (tokens[0] === "POST") {
            const filepath = tokens[1];
            await fs.writeFile(filepath, tokens[2]).then(() => {
                res = "OK";
            }).catch((e)=>{
                res = "FAIL: "+e;
            });
        } else {
            res = "ERROR";
        }
        res+="\n"
        return res;
    }
    let buf = "";
    socket.on('data', async (data)=>{
        buf+=data;
        const lines = buf.split("\n");
        for (let i = 0; i < lines.length -1;++i){
            const res = await processLine(lines[i]);
            socket.write(res);
        }
        buf = lines[lines.length-1];
    });
}).listen('12002');

/*
// Client mock
net.createServer(socket => {

}).listen('12002');
*/
http.create
