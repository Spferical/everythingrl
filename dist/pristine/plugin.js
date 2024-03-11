var REQUESTS = {
    loaded_files: [],
    unique_id: 0
};

// Based on https://macroquad.rs/articles/wasm/
register_plugin = function (importObject) {
    importObject.env.req_post = function (ptr, len) {
            var req_string = UTF8ToString(ptr, len)
            var req = JSON.parse(req_string);
            var file_id = REQUESTS.unique_id;
            var url = req["url"]
            var json_payload = req["json_payload"]
            REQUESTS.unique_id += 1;
            var xhr = new XMLHttpRequest();
            xhr.open('POST', url, true);
            xhr.setRequestHeader('Content-Type', 'application/json');

            xhr.onreadystatechange = function() {
                // looks like readyState === 4 will be fired on either successful or unsuccessful load:
                // https://stackoverflow.com/a/19247992
                if (this.readyState === 4) {
                    var resp;
                    if(this.status === 200) {
                        resp = {'success': this.response}
                    } else {
                        resp = {'error': "" + this.status +  " " + this.response};
                    }
                    console.log(JSON.stringify(resp));
                    var asdf = JSON.stringify(resp);
                    wasm_exports.request_done(file_id, js_object(asdf));
                } 
            };
            xhr.send(json_payload);

            return file_id;

    }
}

// miniquad_add_plugin receive an object with two fields: register_plugin and on_init. Both are functions, both are optional.
miniquad_add_plugin({register_plugin});
