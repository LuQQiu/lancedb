/*
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package com.lancedb.lancedb;

import java.io.Closeable;
import java.util.List;

import io.questdb.jar.jni.JarJniLoader;

/**
 * Represents a connection to the LanceDB database.
 */
public class Connection implements Closeable {
    static {
        JarJniLoader.loadLib(Connection.class, "/nativelib", "lancedb_jni");
    }
    private long nativeConnectHandle;


    private Connection() {}


    /**
     * Creates a new LanceDB connection with the specified database URI.
     *
     * @param databaseUri The URI of the LanceDB database.
     * @return A new Connection object.
     */
    public static native Connection create(String databaseUri);

    /**
     * List all tables in this database, in sorted order.
     */
    public native void tableNames();

    @Override
    public void close() {
        if (nativeConnectHandle != 0) {
            releaseNativeConnection(nativeConnectHandle);
            nativeConnectHandle = 0;
        }
    }

    /**
     * Releases the LanceDB connection resources associated with the given handle.
     *
     * @param handle The native handle to the connection resource.
     */
    private native void releaseNativeConnection(long handle);
}