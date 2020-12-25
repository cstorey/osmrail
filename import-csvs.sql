
DROP TABLE IF EXISTS osm_rel_members;
DROP TABLE IF EXISTS osm_rels;

DROP TABLE IF EXISTS osm_way_nodes;
DROP TABLE IF EXISTS osm_ways;

DROP TABLE IF EXISTS osm_nodes;
-- nodes.write_record(&["node_id", "lat", "lon", "tags"])?;
CREATE TABLE osm_nodes (
    node_id bigint not null,
    lat double precision not null,
    lon double precision not null,
    tags jsonb not null
);

-- ways.write_record(&["way_id", "tags"])?;
CREATE TABLE osm_ways (
    way_id bigint NOT NULL,
    tags jsonb not null
);

-- way_nodes.write_record(&["way_id", "ordinal", "node_id"])?;
CREATE TABLE osm_way_nodes (
    way_id bigint not null,
    ordinal bigint not null,
    node_id bigint not null
);

-- rels.write_record(&["rel_id", "tags"])?;
CREATE TABLE osm_rels (
    rel_id bigint NOT NULL,
    tags jsonb not null
);

-- rel_members.write_record(&["rel_id", "ordinal", "role", "node_id", "way_id", "rel_id"])?;
CREATE TABLE osm_rel_members (
    rel_id bigint not null,
    ordinal bigint not null,
    role text, -- nullable
    member_node_id bigint, -- nullable
    member_way_id bigint, -- nullable
    member_rel_id bigint, -- nullable
    CHECK(num_nonnulls(member_node_id, member_way_id, member_rel_id) = 1)
    -- FOREIGN KEY (rel_id) REFERENCES osm_rels(rel_id),
    -- FOREIGN KEY (member_node_id) REFERENCES osm_nodes(node_id),
    -- FOREIGN KEY (member_way_id) REFERENCES osm_ways(way_id),
    -- FOREIGN KEY (member_rel_id) REFERENCES osm_rels(rel_id)
);


\COPY osm_nodes (node_id, lat, lon, tags) FROM 'csvs/nodes.csv' with CSV HEADER;

\COPY osm_ways (way_id, tags) FROM 'csvs/ways.csv' with CSV HEADER;
\COPY osm_way_nodes (way_id, ordinal, node_id) FROM 'csvs/way-nodes.csv' with CSV HEADER;

\COPY osm_rels (rel_id, tags) FROM 'csvs/relations.csv' with CSV HEADER;
\COPY osm_rel_members (rel_id, ordinal, role, member_node_id, member_way_id, member_rel_id) FROM 'csvs/relation-members.csv' with CSV HEADER;

ALTER TABLE ONLY osm_nodes
    ADD CONSTRAINT osm_nodes_pkey PRIMARY KEY (node_id);

ALTER TABLE ONLY osm_ways
    ADD CONSTRAINT osm_ways_pkey PRIMARY KEY (way_id);
ALTER TABLE ONLY osm_way_nodes
    ADD CONSTRAINT osm_way_nodes_pkey PRIMARY KEY (way_id, ordinal);
ALTER TABLE ONLY osm_way_nodes
    ADD CONSTRAINT osm_way_nodes_node_id_fkey FOREIGN KEY (node_id) REFERENCES osm_nodes(node_id);
ALTER TABLE ONLY osm_way_nodes
    ADD CONSTRAINT osm_way_nodes_way_id_fkey FOREIGN KEY (way_id) REFERENCES osm_ways(way_id);

ALTER TABLE ONLY osm_rels
    ADD CONSTRAINT osm_rels_pkey PRIMARY KEY (rel_id);

ALTER TABLE ONLY osm_rel_members
    ADD CONSTRAINT osm_rel_members_rel_id_fkey FOREIGN KEY (rel_id) REFERENCES osm_rels(rel_id);
ALTER TABLE ONLY osm_rel_members
    ADD CONSTRAINT osm_rel_members_pkey PRIMARY KEY (rel_id, ordinal);

-- Normally, these would make sense if we have a complete import. However,
-- because we might only import a subset (eg: London), there might well be
-- missing references. We might drop the offending rows instead, but not for
-- now.
--
-- ALTER TABLE ONLY osm_rel_members
--     ADD CONSTRAINT osm_rel_members_member_node_id_fkey FOREIGN KEY (member_node_id) REFERENCES osm_nodes(node_id);
-- ALTER TABLE ONLY osm_rel_members
--     ADD CONSTRAINT osm_rel_members_member_rel_id_fkey FOREIGN KEY (member_rel_id) REFERENCES osm_rels(rel_id);
-- ALTER TABLE ONLY osm_rel_members
--     ADD CONSTRAINT osm_rel_members_member_way_id_fkey FOREIGN KEY (member_way_id) REFERENCES osm_ways(way_id);

CREATE INDEX osm_nodes_tags_idx ON osm_nodes USING gin (tags);
CREATE INDEX osm_ways_tags_idx ON osm_ways USING gin (tags);
CREATE INDEX osm_rels_tags_idx ON osm_rels USING gin (tags);
