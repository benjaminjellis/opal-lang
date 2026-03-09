-module(zier_unknown_helpers).
-export([from/1, run/2, string/0, int/0, float/0, bool/0, list/1, field/2]).

from(Value) ->
    Value.

run(Data, Decoder) ->
    Decoder(Data).

string() ->
    fun(Data) when is_binary(Data) ->
        {ok, Data};
       (Data) ->
        {error, [decode_error(<<"String">>, Data)]}
    end.

int() ->
    fun(Data) when is_integer(Data) ->
        {ok, Data};
       (Data) ->
        {error, [decode_error(<<"Int">>, Data)]}
    end.

float() ->
    fun(Data) when is_float(Data) ->
        {ok, Data};
       (Data) ->
        {error, [decode_error(<<"Float">>, Data)]}
    end.

bool() ->
    fun(true) ->
        {ok, true};
       (false) ->
        {ok, false};
       (Data) ->
        {error, [decode_error(<<"Bool">>, Data)]}
    end.

list(ItemDecoder) ->
    fun(Data) when is_list(Data) ->
        decode_list(Data, ItemDecoder, []);
       (Data) ->
        {error, [decode_error(<<"List">>, Data)]}
    end.

field(Key, ValueDecoder) ->
    fun(Data) when is_map(Data) ->
        case maps:find(Key, Data) of
            {ok, Value} ->
                ValueDecoder(Value);
            error ->
                {error, [{decodeerror, <<"Field">>, <<"Nothing">>}]}
        end;
       (Data) ->
        {error, [decode_error(<<"Map">>, Data)]}
    end.

decode_list([], _ItemDecoder, Acc) ->
    {ok, lists:reverse(Acc)};
decode_list([Item | Rest], ItemDecoder, Acc) ->
    case ItemDecoder(Item) of
        {ok, Decoded} ->
            decode_list(Rest, ItemDecoder, [Decoded | Acc]);
        {error, Errors} ->
            {error, Errors}
    end.

decode_error(Expected, Data) ->
    {decodeerror, Expected, classify(Data)}.

classify(true) ->
    <<"Bool">>;
classify(false) ->
    <<"Bool">>;
classify(unit) ->
    <<"Unit">>;
classify(Data) when is_integer(Data) ->
    <<"Int">>;
classify(Data) when is_float(Data) ->
    <<"Float">>;
classify(Data) when is_binary(Data) ->
    <<"String">>;
classify(Data) when is_list(Data) ->
    <<"List">>;
classify(Data) when is_map(Data) ->
    <<"Map">>;
classify(Data) when is_tuple(Data) ->
    <<"Tuple">>;
classify(Data) when is_pid(Data) ->
    <<"Pid">>;
classify(Data) when is_function(Data) ->
    <<"Function">>;
classify(_Data) ->
    <<"Unknown">>.
