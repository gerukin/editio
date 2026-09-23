-module(greeting).
-export([hello/1]).
%% Pattern matching with a guard.
hello(Name) when is_list(Name) ->
    io:format("Hello, ~s!~n", [Name]).
