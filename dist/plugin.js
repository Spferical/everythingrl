var REQUESTS = {
    loaded_files: [],
    unique_id: 0
};

// Based on https://macroquad.rs/articles/wasm/
register_plugin = function (importObject) {
    importObject.env.make_put_request = function (url_ptr, url_len) {
            var url = UTF8ToString(ptr, len);
            var file_id = REQUESTS.unique_id;
            REQUESTS.unique_id += 1;
            var xhr = new XMLHttpRequest();
            xhr.open('PUT', url, true);
            xhr.responseType = 'arraybuffer'; 

            xhr.onreadystatechange = function() {
	        // looks like readyState === 4 will be fired on either successful or unsuccessful load:
		// https://stackoverflow.com/a/19247992
                if (this.readyState === 4) {
                    if(this.status === 200) {  
                        var uInt8Array = new Uint8Array(this.response);
    
                        FS.loaded_files[file_id] = uInt8Array;
                        wasm_exports.file_loaded(file_id);
                    } else {
                        FS.loaded_files[file_id] = null;
                        wasm_exports.file_loaded(file_id);
                    }
                } 
            };
            xhr.send();

            return file_id;

    }
}

// miniquad_add_plugin receive an object with two fields: register_plugin and on_init. Both are functions, both are optional.
miniquad_add_plugin({register_plugin});
