CREATE TABLE absences (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp DATETIME NOT NULL,
  duration INT NOT NULL,   -- in seconds
  begin_beat BIGINT NOT NULL REFERENCES beats(id) ON DELETE CASCADE,
  end_beat BIGINT NOT NULL REFERENCES beats(id) ON DELETE CASCADE
);
