# This script takes in a MYSQL db dump and outputs a script that will produce a "compress" format SQLITE db.

import sys
import os
from tqdm import tqdm
import subprocess

sourcepath = (
    sys.argv[1] if len(sys.argv) > 1 else r"C:\Users\J\Downloads\libgren\libgen.sql"
)

print(f"Source file: {sourcepath}")

structspath = os.path.splitext(sourcepath)[0] + ".struct.sql"

SCHEMA_WITHOUT_INSERTS = True

if SCHEMA_WITHOUT_INSERTS:
    with open(structspath, "w", encoding="utf-8") as struct:
        with open(sourcepath, "r", encoding="utf-8", errors="replace") as source:
            insert_count = 0
            for line in tqdm(source):

                if line.startswith("INSERT INTO"):
                    insert_count += 1
                elif insert_count > 0:
                    struct.write(f"/* INSERT INTO × {insert_count}*/\n")
                    insert_count = 0
                    struct.write(line)
                else:
                    struct.write(line)
    print(f"Structure Done. Result file: {structspath}")


def convert_sql(code):
    # mysql2sqlite.sh is an awk script. Use a file for input and capture output.
    with open("temp.sql", "w", encoding="utf-8") as temp:
        temp.write(code)
    try:
        result = subprocess.run(
            ["bash", "-c", "./mysql2sqlite.sh temp.sql"],
            capture_output=True,
        ).stdout.decode("utf-8")
    except:
        print("problematic text in temp.sql")
        raise
    os.remove("temp.sql")
    return result


sqlitepath = os.path.splitext(sourcepath)[0] + ".sqlite.sql"
with open(sqlitepath, "w", encoding="utf-8") as sqlite:
    accumulator = ""
    count = 0
    with open(sourcepath, "r", encoding="utf-8", errors="replace") as source:
        for line in tqdm(source, total=20000):
            # ignore description and description_edited
            if line.startswith("INSERT INTO `descr"):
                continue
            if line.startswith("INSERT ") and count > 100:
                result = convert_sql(accumulator)
                sqlite.write(result)
                # sqlite.flush()
                accumulator = line
                count = 1
            else:
                accumulator += line
                count += 1
        sqlite.write(convert_sql(accumulator))

print(f"SQLITE Done. Result file: {sqlitepath}")

print(f"""
Create the sql db with: 
      
    sqlite3 libgen.db < {sqlitepath}

and then compress it using the appropriate compress sql, for example:

    sqlite3 libgen.db < compress-fiction.sql
""")
