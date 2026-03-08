-module(zier_list_helpers).
-export([foldl/3, foldr/3]).

%% Adapt Zier's curried fun(X) -> fun(Acc) -> ... end
%% to the 2-arity fun(X, Acc) -> ... end that lists:foldl/foldr expect.

foldl(Fun, Acc0, List) ->
    lists:foldl(fun(X, Acc) -> (Fun(X))(Acc) end, Acc0, List).

foldr(Fun, Acc0, List) ->
    lists:foldr(fun(X, Acc) -> (Fun(X))(Acc) end, Acc0, List).
