const fs = require("fs");

function sf(code, i) {
    let d = 1;
    while (d > 0) {
        i++;
        const x = code.charCodeAt(i);
        if (x === 91) d++;
        if (x === 93) d--;
    }
    return i;
}

function sb(code, i) {
    let d = 1;
    while (d > 0) {
        i--;
        const x = code.charCodeAt(i);
        if (x === 93) d++;
        if (x === 91) d--;
    }
    return i;
}

function run(code) {
    const mem = new Int32Array(30000);
    let dp = 0, i = 0;
    while (i < code.length) {
        const ch = code.charCodeAt(i);
        if (ch === 62) dp++;
        else if (ch === 60) dp--;
        else if (ch === 43) mem[dp]++;
        else if (ch === 45) mem[dp]--;
        else if (ch === 46) process.stdout.write(String.fromCharCode(mem[dp]));
        else if (ch === 44) mem[dp] = fs.readSync(0, Buffer.alloc(1), 0, 1, null) || 0;
        else if (ch === 91 && mem[dp] === 0) i = sf(code, i);
        else if (ch === 93 && mem[dp] !== 0) i = sb(code, i);
        i++;
    }
}

if (process.argv.length < 3) {
    console.log("usage: bf CODE");
    process.exit(1);
}
run(process.argv[2]);
